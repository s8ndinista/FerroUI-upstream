use super::snippet::Placeholder;
use super::SnippetError;
use crate::document::{AnchorMovementType,DocumentSubscription,SimpleSegment,TextAnchor,TextDocument};
use std::cell::{Cell,RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

struct ActiveElement { id:usize,start:TextAnchor,end:TextAnchor }
impl ActiveElement {
    fn segment(&self)->Option<SimpleSegment> {
        let start=self.start.offset().ok()?; let end=self.end.offset().ok()?;
        (end>=start).then(||SimpleSegment::new(start,end-start))
    }
}
struct SessionInner {
    document:TextDocument,
    elements:Vec<ActiveElement>,
    tab_order:Vec<usize>,
    active:Cell<Option<usize>>,
    caret:TextAnchor,
    closed:Cell<bool>,
    syncing:Cell<bool>,
    last_text:RefCell<String>,
    subscription:RefCell<Option<DocumentSubscription>>,
    error:RefCell<Option<String>>,
}
impl Drop for SessionInner { fn drop(&mut self) { if let Some(subscription)=self.subscription.get_mut().take() { subscription.dispose(); } } }

/// A linked-placeholder insertion context. Edits through any host to the active
/// editable element propagate to its bound occurrences after the document's
/// transaction completes. The mirror edits continue the original undo group.
#[derive(Clone)]
pub struct SnippetSession { inner:Rc<SessionInner> }
impl SnippetSession {
    pub(crate) fn new(document:TextDocument,offset:usize,placeholders:Vec<Placeholder>,caret:usize)->Result<Self,SnippetError> {
        let mut elements=Vec::new(); let mut order=BTreeMap::new();
        for placeholder in placeholders {
            let start=document.create_anchor(offset+placeholder.start)?;
            let end=document.create_anchor(offset+placeholder.end)?;
            start.set_movement_type(AnchorMovementType::BeforeInsertion); end.set_movement_type(AnchorMovementType::AfterInsertion);
            start.set_survive_deletion(true); end.set_survive_deletion(true);
            if placeholder.editable { order.entry(placeholder.id).or_insert(elements.len()); }
            elements.push(ActiveElement {id:placeholder.id,start,end});
        }
        let caret=document.create_anchor(offset+caret)?;
        caret.set_movement_type(AnchorMovementType::AfterInsertion); caret.set_survive_deletion(true);
        let tab_order:Vec<_>=order.into_values().collect();
        let active=(!tab_order.is_empty()).then_some(0);
        let inner=Rc::new(SessionInner {document:document.clone(),elements,tab_order,active:Cell::new(active),caret,
            closed:Cell::new(active.is_none()),syncing:Cell::new(false),last_text:RefCell::new(String::new()),subscription:RefCell::new(None),error:RefCell::new(None)});
        let session=Self {inner}; session.refresh_last_text();
        let weak=Rc::downgrade(&session.inner);
        let subscription=document.text_changed(move || {
            if let Some(inner)=weak.upgrade() {
                let session=SnippetSession {inner};
                if let Err(error)=session.synchronize() { *session.inner.error.borrow_mut()=Some(error.to_string()); session.finish(); }
            }
        });
        *session.inner.subscription.borrow_mut()=Some(subscription);
        Ok(session)
    }
    pub fn is_closed(&self)->bool { self.inner.closed.get() }
    pub fn error(&self)->Option<String> { self.inner.error.borrow().clone() }
    pub fn active_segment(&self)->Option<SimpleSegment> {
        if self.is_closed() {return None;}
        self.inner.elements.get(*self.inner.tab_order.get(self.inner.active.get()?)?)?.segment()
    }
    pub fn active_placeholder(&self)->Option<usize> {
        self.inner.elements.get(*self.inner.tab_order.get(self.inner.active.get()?)?).map(|e|e.id)
    }
    pub fn caret_offset(&self)->Option<usize> { self.inner.caret.offset().ok() }
    pub fn next_element(&self,backwards:bool)->Option<SimpleSegment> {
        let index=self.inner.active.get()?;
        let next=if backwards {index.saturating_sub(1)} else {index+1};
        if next>=self.inner.tab_order.len() {self.finish();return None;}
        self.inner.active.set(Some(next)); self.refresh_last_text(); self.active_segment()
    }
    pub fn finish(&self) { self.inner.closed.set(true); self.inner.active.set(None); }
    pub fn replace_active(&self,text:&str)->Result<(),SnippetError> {
        let segment=self.active_segment().ok_or(SnippetError::Closed)?;
        let _update=self.inner.document.begin_update()?;
        self.inner.document.replace(segment.offset,segment.length,text)?;
        Ok(())
    }
    fn refresh_last_text(&self) {
        if let Some(segment)=self.active_segment() {
            if let Ok(text)=self.inner.document.get_text(segment.offset,segment.length) { *self.inner.last_text.borrow_mut()=text; }
        }
    }
    fn synchronize(&self)->Result<(),SnippetError> {
        if self.is_closed() || self.inner.syncing.get() {return Ok(());}
        let Some(segment)=self.active_segment() else {self.finish();return Ok(());};
        let text=self.inner.document.get_text(segment.offset,segment.length)?;
        if *self.inner.last_text.borrow()==text {return Ok(());}
        *self.inner.last_text.borrow_mut()=text.clone();
        // Undo/redo already contain the mirror operations and must replay them
        // without causing additional changes from this listener.
        if !self.inner.document.accept_changes() {return Ok(());}
        let active=self.inner.tab_order[self.inner.active.get().unwrap()];
        let id=self.inner.elements[active].id;
        let mut ranges:Vec<_>=self.inner.elements.iter().enumerate().filter(|(i,e)|*i!=active && e.id==id).filter_map(|(_,e)|e.segment()).collect();
        ranges.sort_by_key(|r|r.offset);
        if ranges.is_empty() {return Ok(());}
        self.inner.syncing.set(true);
        struct Reset<'a>(&'a Cell<bool>);
        impl Drop for Reset<'_> {fn drop(&mut self){self.0.set(false);}}
        let _reset=Reset(&self.inner.syncing);
        let _update=self.inner.document.begin_continued_update()?;
        for range in ranges.into_iter().rev() {self.inner.document.replace(range.offset,range.length,&text)?;}
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snippets::Snippet;
    #[test]
    fn bound_elements_mirror_external_edits_in_one_undo_group() {
        let document=TextDocument::new();
        let snippet=Snippet::parse("let ${1:name} = ${2:value}; $1$0").unwrap();
        let session=snippet.insert(document.clone(),0,"").unwrap();
        assert_eq!(document.text(),"let name = value; name");
        let segment=session.active_segment().unwrap();
        document.replace(segment.offset,segment.length,"😀").unwrap();
        assert_eq!(document.text(),"let 😀 = value; 😀");
        assert!(document.undo().unwrap()); assert_eq!(document.text(),"let name = value; name");
        assert!(document.redo().unwrap()); assert_eq!(document.text(),"let 😀 = value; 😀");
        assert_eq!(session.next_element(false).unwrap().length,5);
        session.replace_active("42").unwrap(); assert_eq!(document.text(),"let 😀 = 42; 😀");
        assert!(session.next_element(false).is_none()); assert!(session.is_closed());
    }
    #[test]
    fn missing_defaults_and_parse_errors() {
        let document=TextDocument::new();
        let session=Snippet::parse("$1-$1$0").unwrap().insert(document.clone(),0,"").unwrap();
        session.replace_active("x").unwrap(); assert_eq!(document.text(),"x-x");
        assert!(Snippet::parse("${name}").is_err()); assert!(Snippet::parse("${1:broken").is_err());
    }
}
