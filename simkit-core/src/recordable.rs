// pub trait Recordable {
//     type Op;

//     fn get_recording(&self) -> Vec<Self::Op>;
//     fn apply_recording(&mut self, recording: Vec<Self::Op>);
// }

// pub trait RecordableCell<T: Recordable> {
//     fn set(&mut self, value: T);

// pub trait RecordableMap<K, V: Recordable> {
//     fn insert(&mut self, key: K, value: V);
//     fn remove(&mut self, key: K) -> Option<V>;
// }

// pub trait RecordableVec<T: Recordable> {
//     fn get(&self, index: usize) -> Option<&T>;
//     fn get_mut(&mut self, index: usize) -> Option<&mut T>;
//     fn push(&mut self, value: T);
//     fn pop(&mut self) -> Option<T>;
//     fn clear(&mut self);
//     fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut T> + 'a
//     where
//         T: 'a;
// }
