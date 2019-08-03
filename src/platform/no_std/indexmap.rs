pub mod map {
    use crate::platform::prelude::*;

    #[derive(Default, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct IndexMap<K, V>(Vec<(K, V)>);
    pub struct Iter<'a, K, V>(core::slice::Iter<'a, (K, V)>);

    impl<K: PartialEq, V> IndexMap<K, V> {
        pub fn insert(&mut self, key: K, value: V) {
            if let Some(index) = self.0.iter().position(|(k, _)| k == &key) {
                self.0[index] = (key, value);
            } else {
                self.0.push((key, value));
            }
        }

        pub fn remove<K2>(&mut self, key: &K2)
        where
            K: core::borrow::Borrow<K2>,
            K2: ?Sized + PartialEq,
        {
            if let Some(index) = self.0.iter().position(|(k, _)| k.borrow() == key) {
                self.0.remove(index);
            }
        }

        pub fn iter(&self) -> Iter<'_, K, V> {
            Iter(self.0.iter())
        }

        pub fn clear(&mut self) {
            self.0.clear();
        }
    }

    impl<'a, K, V> Iterator for Iter<'a, K, V> {
        type Item = (&'a K, &'a V);
        fn next(&mut self) -> Option<Self::Item> {
            self.0.next().map(|(a, b)| (a, b))
        }
    }
}
