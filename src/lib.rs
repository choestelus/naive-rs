mod naive_vec;

#[cfg(test)]
mod tests {
    use crate::naive_vec::NaiveVec;

    #[test]
    fn zero_sized_type_vector_push_pop_works_correctly() {
        let mut v: NaiveVec<()> = NaiveVec::new();
        v.push(());
        v.push(());
        v.push(());
        v.push(());
        let elem = v.pop();
        assert_eq!(elem, Some(()));
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn iter_vec_works_correctly() {
        let mut v: NaiveVec<()> = NaiveVec::new();
        v.push(());
        v.push(());
        v.push(());
        v.push(());

        let mut iter = v.iter().enumerate();
        assert_eq!(iter.next(), Some((0 as usize, &())));
        assert_eq!(iter.next(), Some((1 as usize, &())));
        assert_eq!(iter.next(), Some((2 as usize, &())));
        assert_eq!(iter.next(), Some((3 as usize, &())));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn insert_works_correctly() {
        let mut v: NaiveVec<i64> = NaiveVec::new();
        v.push(1);
        v.push(2);
        v.push(3);
        v.push(4);
        v.insert(2, 8);
        assert_eq!(v[2], 8);
        assert_eq!(v.len(), 5);
    }

    #[test]
    fn remove_works_correctly() {
        let mut v: NaiveVec<i64> = NaiveVec::new();
        v.push(1);
        v.push(2);
        v.push(3);
        v.push(4);
        let elem = v.remove(1);
        assert_eq!(elem, 2);
        assert_eq!(v.len(), 3);
    }
}
