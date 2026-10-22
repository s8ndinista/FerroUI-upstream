use super::{CompletionList,ICompletionData};
use crate::document::{AnchorMovementType,DocumentError,SimpleSegment,TextAnchor,TextDocument};

pub struct CompletionSession {
    document:TextDocument,
    start:TextAnchor,
    end:TextAnchor,
    pub list:CompletionList,
    pub close_automatically:bool,
    pub close_when_caret_at_beginning:bool,
    closed:bool,
}
impl CompletionSession {
    pub fn new(document:TextDocument,start:usize,end:usize,list:CompletionList)->Result<Self,DocumentError> {
        // Validate the entire segment even when the endpoints individually exist.
        if start>end {return Err(DocumentError::InvalidRange {offset:start,length:0,text_length:document.text_length()});}
        document.get_text(start,end.saturating_sub(start))?;
        let start=document.create_anchor(start)?; let end=document.create_anchor(end)?;
        start.set_movement_type(AnchorMovementType::BeforeInsertion);
        end.set_movement_type(AnchorMovementType::AfterInsertion);
        Ok(Self { document,start,end,list,close_automatically:true,close_when_caret_at_beginning:false,closed:false })
    }
    pub fn segment(&self)->Option<SimpleSegment> {
        let start=self.start.offset().ok()?; let end=self.end.offset().ok()?;
        (start<=end).then(||SimpleSegment::new(start,end-start))
    }
    pub fn is_closed(&self)->bool { self.closed || self.segment().is_none() }
    pub fn close(&mut self) { self.closed=true; }
    pub fn caret_moved(&mut self,caret:usize)->Result<(),DocumentError> {
        let Some(segment)=self.segment() else { self.closed=true; return Ok(()); };
        if caret<segment.offset || caret>segment.end_offset() || caret==segment.offset && self.close_when_caret_at_beginning {
            if self.close_automatically { self.closed=true; }
            return Ok(());
        }
        let prefix=self.document.get_text(segment.offset,caret-segment.offset)?;
        self.list.select_item(&prefix); Ok(())
    }
    pub fn request_insertion(&mut self)->Result<bool,DocumentError> {
        if self.is_closed() { return Ok(false); }
        let Some(segment)=self.segment() else { return Ok(false); };
        let Some(item)=self.list.selected_item().cloned() else { return Ok(false); };
        let _update=self.document.begin_update()?;
        item.complete(&self.document,segment)?;
        self.closed=true; Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::CompletionItem;
    #[test]
    fn insertion_tracks_prefix_and_external_edits() {
        let doc=TextDocument::from_text("x Re");
        let mut session=CompletionSession::new(doc.clone(),2,4,CompletionList::new(vec![CompletionItem::new("ReadLine")])).unwrap();
        doc.insert(4,"a").unwrap(); session.caret_moved(5).unwrap();
        doc.insert(0,"😀").unwrap();
        assert_eq!(session.segment().unwrap(),SimpleSegment::new(4,3));
        assert!(session.request_insertion().unwrap()); assert_eq!(doc.text(),"😀x ReadLine");
        assert!(session.is_closed());
    }
}
