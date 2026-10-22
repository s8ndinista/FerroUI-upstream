use super::{HighlightedLine,HighlightingEngine,HighlightingError,HighlightingState};
use crate::document::{DocumentSubscription,TextDocument};
use std::cell::{Cell,RefCell};
use std::rc::Rc;

/// An incremental line highlighter. Unaffected lines retain their parsed state;
/// changed lines are recomputed lazily through the requested line.
pub struct DocumentHighlighter {
    document:TextDocument,
    engine:HighlightingEngine,
    cache:RefCell<Vec<HighlightedLine>>,
    invalid_from:Rc<Cell<usize>>,
    subscription:DocumentSubscription,
}
impl DocumentHighlighter {
    pub fn new(document:TextDocument,engine:HighlightingEngine)->Self{
        let invalid_from=Rc::new(Cell::new(0));let weak=Rc::downgrade(&invalid_from);let observed=document.clone();
        let subscription=document.changing(move|change|{
            if let Some(invalid)=weak.upgrade(){let line=observed.get_line_by_offset(change.offset).map_or(0,|l|l.line_number-1);invalid.set(invalid.get().min(line));}
        });
        Self{document,engine,cache:RefCell::new(Vec::new()),invalid_from,subscription}
    }
    pub fn highlight_line(&self,line_number:usize)->Result<HighlightedLine,HighlightingError>{
        if line_number==0 || line_number>self.document.line_count(){return Err(HighlightingError::InvalidDefinition("Line number lies outside the document".into()));}
        let invalid=self.invalid_from.get();
        self.cache.borrow_mut().truncate(invalid);
        while self.cache.borrow().len()<line_number{
            let number=self.cache.borrow().len()+1;
            let line=self.document.get_line_by_number(number).map_err(|e|HighlightingError::InvalidDefinition(e.to_string()))?;
            let text=self.document.get_text(line.offset,line.length).map_err(|e|HighlightingError::InvalidDefinition(e.to_string()))?;
            let state=self.cache.borrow().last().map(|l|l.end_state.clone()).unwrap_or_default();
            let highlighted=self.engine.highlight_line(&text,number,line.offset,&state)?;
            self.cache.borrow_mut().push(highlighted);
        }
        self.invalid_from.set(self.cache.borrow().len());
        Ok(self.cache.borrow()[line_number-1].clone())
    }
    pub fn invalidate_highlighting(&self){self.invalid_from.set(0);}
    pub fn get_color_stack(&self,line_number:usize)->Result<HighlightingState,HighlightingError>{if line_number==0{Ok(HighlightingState::default())}else{Ok(self.highlight_line(line_number)?.end_state)}}
}
impl Drop for DocumentHighlighter{fn drop(&mut self){self.subscription.dispose();}}
