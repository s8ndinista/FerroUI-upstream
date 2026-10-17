use crate::*;
use std::collections::{BTreeMap,BTreeSet,HashMap,HashSet};
/// Uniform-grid broadphase; oversized shapes use a bounded-cost separate set.
#[derive(Clone,Debug)]
pub struct SpatialIndex{cell_size:f64,cells:HashMap<(i32,i32),HashSet<NodeId>>,bounds:BTreeMap<NodeId,Rect>,large:HashSet<NodeId>}
impl Default for SpatialIndex{fn default()->Self{Self::new(256.0)}}
impl SpatialIndex{
pub fn new(cell_size:f64)->Self{Self{cell_size:cell_size.max(1.0),cells:HashMap::new(),bounds:BTreeMap::new(),large:HashSet::new()}}
pub fn len(&self)->usize{self.bounds.len()}pub fn is_empty(&self)->bool{self.bounds.is_empty()}
pub fn clear(&mut self){self.cells.clear();self.bounds.clear();self.large.clear()}
fn range(&self,r:Rect)->(i32,i32,i32,i32){((r.x/self.cell_size).floor()as i32,(r.y/self.cell_size).floor()as i32,(r.right()/self.cell_size).floor()as i32,(r.bottom()/self.cell_size).floor()as i32)}
fn cell_count((a,b,c,d):(i32,i32,i32,i32))->u64{((c as i64-a as i64+1).max(0)as u64).saturating_mul((d as i64-b as i64+1).max(0)as u64)}
pub fn insert(&mut self,id:NodeId,bounds:Rect){self.remove(id);if !bounds.is_finite(){return}self.bounds.insert(id,bounds);let range=self.range(bounds);if Self::cell_count(range)>1024{self.large.insert(id);return}let(x0,y0,x1,y1)=range;for x in x0..=x1{for y in y0..=y1{self.cells.entry((x,y)).or_default().insert(id);}}}
pub fn remove(&mut self,id:NodeId){let Some(bounds)=self.bounds.remove(&id)else{return};if self.large.remove(&id){return}let(x0,y0,x1,y1)=self.range(bounds);for x in x0..=x1{for y in y0..=y1{if let Some(cell)=self.cells.get_mut(&(x,y)){cell.remove(&id);if cell.is_empty(){self.cells.remove(&(x,y));}}}}}
pub fn bounds(&self,id:NodeId)->Option<Rect>{self.bounds.get(&id).copied()}
pub fn query(&self,rect:Rect)->Vec<NodeId>{if !rect.is_finite(){return Vec::new()}let range=self.range(rect);if Self::cell_count(range)>16_384{return self.bounds.iter().filter_map(|(id,b)|b.intersects(rect).then_some(*id)).collect()}let mut ids:BTreeSet<_>=self.large.iter().copied().collect();let(x0,y0,x1,y1)=range;for x in x0..=x1{for y in y0..=y1{if let Some(cell)=self.cells.get(&(x,y)){ids.extend(cell.iter().copied())}}}ids.into_iter().filter(|id|self.bounds.get(id).is_some_and(|b|b.intersects(rect))).collect()}
}
#[derive(Clone,Debug,Default)]
pub struct SceneIndex{pub spatial:SpatialIndex,pub transforms:BTreeMap<NodeId,Transform>,pub draw_order:Vec<NodeId>,pub z_order:BTreeMap<NodeId,usize>,pub revision:u64}
impl SceneIndex{
pub fn rebuild(&mut self,d:&Document,page:NodeId){self.spatial.clear();self.transforms.clear();self.draw_order.clear();self.z_order.clear();if let Some(p)=d.page(page){let mut stack:Vec<_>=p.children.iter().rev().map(|id|(*id,Transform::IDENTITY,true,None::<Rect>,false)).collect();while let Some((id,parent,visible,clip,operand))=stack.pop(){let Some(n)=d.nodes.get(&id)else{continue};let transform=parent*n.local_transform();self.transforms.insert(id,transform);let visible=visible&&n.visible;let bounds=transform.transform_rect(n.local_bounds());if visible&&!operand{let mut paint=bounds;for s in n.strokes.iter().filter(|s|s.visible){paint=paint.inflate(s.width.max(0.0))}for e in n.effects.iter().filter(|e|e.visible){paint=paint.union(Rect::new(bounds.x+e.offset.x,bounds.y+e.offset.y,bounds.width,bounds.height).inflate(e.radius*2.0+e.spread.max(0.0)))}if let Some(clip)=clip{if let Some(i)=paint.intersection(clip){self.spatial.insert(id,i)}}else{self.spatial.insert(id,paint)}self.z_order.insert(id,self.draw_order.len());self.draw_order.push(id)}let child_clip=if n.clip_content{match clip{Some(c)=>c.intersection(bounds),None=>Some(bounds)}}else{clip};let child_visible=visible&&(!n.clip_content||child_clip.is_some());for child in n.children.iter().rev(){stack.push((*child,transform,child_visible,child_clip,operand||n.kind==NodeKind::Boolean))}}}self.revision=d.revision}
pub fn visible_nodes(&self,r:Rect)->Vec<NodeId>{let mut ids=self.spatial.query(r);ids.sort_unstable_by_key(|id|self.z_order.get(id).copied().unwrap_or(0));ids}
pub fn hit_test(&self,d:&Document,p:Point,tolerance:f64,include_locked:bool)->Option<NodeId>{let mut ids=self.spatial.query(Rect::new(p.x-tolerance,p.y-tolerance,tolerance*2.0,tolerance*2.0));ids.sort_unstable_by_key(|id|std::cmp::Reverse(self.z_order.get(id).copied().unwrap_or(0)));for id in ids{let Some(n)=d.nodes.get(&id)else{continue};if !include_locked&&d.effective_locked(id){continue}let mut clipped=false;let mut parent=n.parent;while let Some(pid)=parent{let Some(n)=d.nodes.get(&pid)else{break};if n.clip_content{if let Some(inv)=self.transforms.get(&pid).and_then(|t|t.inverse()){if !n.hit_test_local(inv.apply(p),0.0){clipped=true;break}}}parent=n.parent}if !clipped&&d.hit_test_node(id,p,tolerance){return Some(id)}}None}
}

