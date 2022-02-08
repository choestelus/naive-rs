#[allow(unused)]
use std::{ptr::NonNull, marker::PhantomData};
use std::ops::{Deref, DerefMut};
use std::alloc;
use std::ptr;
use std::alloc::{Layout, alloc, realloc};

// RawVec<T> was separated from NaiveVec<T>
// since there are overlapping functionalities when implementing
// IntoIter trait
struct RawVec<T> {
    ptr: NonNull<T>,
    cap: usize,
    // for explicit drop-check analysis
    // ossociate dropping over NaiveVec<T> with dropping over T
    _marker: PhantomData<T>,
}

unsafe impl<T: Send> Send for RawVec<T> {}
unsafe impl<T: Sync> Sync for RawVec<T> {}

impl<T> RawVec<T> {
    fn new() -> Self {
        // check if T is zero-sized type or not then set cap to usize::MAX
        // since every operation on ptr with zero-sized type is no-op
        // to guard against capacity overflow.
        let cap = if std::mem::size_of::<T>() == 0 { usize::MAX } else { 0 };
        RawVec {
            ptr: NonNull::dangling(),
            cap: cap,
            _marker: PhantomData
        }
    }


    // grow is where actual allocation happens.
    fn grow(&mut self) {

        // for zero-sized type, any operation should not reach here
        // thus, to call grow() on zero-sized type is invalid and rejected here.
        assert!(std::mem::size_of::<T>() != 0, "capacity overflow");

        // part 1: create memory layout for allocation from set cap
        let (new_cap, new_layout) = if self.cap == 0 {
            (1, Layout::array::<T>(1).unwrap())
        } else {
            let new_cap = self.cap * 2;
            let new_layout = Layout::array::<T>(new_cap).unwrap();
            (new_cap, new_layout)
        };

        // need to check against isize::MAX here as LLVM's GEP instruction
        // use signed integer, thus limitations are reflected here as well.
        assert!(new_layout.size() <= isize::MAX as usize, "grow allocation is too large");

        // part 2: actual allocation
        let new_ptr = if self.cap == 0 {
            unsafe { alloc(new_layout) }
        } else {
            // unwrap() here should never fail since it checks if number of bytes is <= usize::MAX
            // but layout created here always passed assertion with <= isize::MAX above
            let old_layout = Layout::array::<T>(self.cap).unwrap();
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { realloc(old_ptr, old_layout, new_layout.size()) }
        };

        // abort if allocation fails, using alloc error handler provided by std::alloc
        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(ptr) => ptr,
            None => alloc::handle_alloc_error(new_layout),
        };
        self.cap = new_cap;
    }
    
}

impl<T> Drop for RawVec<T> {
    // no-op if dropping on zero-sized type or unallocated pointer.
    fn drop(&mut self) {
        let elem_size = std::mem::size_of::<T>();
        if self.cap == 0 || elem_size == 0 {
            return;
        }

        let layout = Layout::array::<T>(self.cap).unwrap();
        unsafe { alloc::dealloc(self.ptr.as_ptr() as *mut u8, layout); }
    }
}

pub struct NaiveVec<T> {
    buf: RawVec<T>,
    len: usize,
}

impl<T> NaiveVec<T> {
    fn ptr(&self) -> *mut T {
        self.buf.ptr.as_ptr()
    }
    fn cap(&self) -> usize {
        self.buf.cap
    }

    pub fn new() -> Self {
        NaiveVec { buf: RawVec::new(), len: 0 }
    }

    pub fn push(&mut self, elem: T) {
        if self.len == self.cap() {
            self.buf.grow();
        }
        unsafe {
            // since ptr points at beginning of memory allocated
            // we'll do poiter arithmetic here to point at last element as beginning
            // for new element to be pushed into.
            ptr::write(self.ptr().add(self.len), elem);
        }

        // increment length after concatenated element into collection
        self.len = self.len + 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        match self.len {
            0 => None,
            _ => {
                self.len = self.len - 1;
                unsafe { Some(ptr::read(self.ptr().add(self.len)))}
            }
        }
    }

    pub fn insert(&mut self, index: usize, elem: T) {
        if index > self.len {
            panic!("index out of bounds");
        }
        if self.cap() == self.len { self.buf.grow(); }
        unsafe {
            // copy from old index, make 1 element space, then set destination to shifted index
            // then write where index is.
            let p = self.ptr().add(index);
            ptr::copy(p, p.add(1), self.len - index);
            ptr::write(p, elem);
        }
        self.len += 1;
    }

    pub fn remove(&mut self, index: usize) -> T {
        if index >= self.len {
            panic!("index out of bounds");
        }

        self.len -= 1;
        unsafe {
            // read element as result for return value
            // then shift back element, copy and replace where old element was
            let p = self.ptr().add(index);
            let result = ptr::read(p);
            ptr::copy(p.add(1), p, self.len - index);

            result
        }
    }

}

struct RawValIter<T> {
    start: *const T,
    end: *const T,
}

impl<T> RawValIter<T> {
    unsafe fn new(slice: &[T]) -> Self {
        RawValIter {
            start: slice.as_ptr(),
            end: if std::mem::size_of::<T>() == 0 {
                ((slice.as_ptr() as usize) + slice.len()) as *const T
            } else if slice.len() == 0 {
                slice.as_ptr()
            } else {
                slice.as_ptr().add(slice.len())
            }
        }
    }
}

impl<T> Iterator for RawValIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                let result = ptr::read(self.start);
                self.start = if std::mem::size_of::<T>() == 0 {
                    (self.start as usize + 1) as *const T
                } else {
                    self.start.offset(1)
                };
                Some(result)
            }
        }
        
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let elem_size = std::mem::size_of::<T>();
        let divider = if elem_size == 0 { 1 } else { elem_size };
        let len = (self.end as usize - self.start as usize) / divider;
        (len, Some(len))
    }
}

impl<T> Drop for NaiveVec<T> {
    fn drop(&mut self) {
        // in example, it calls pop until None is yielded
        // but here we set len = 0 instead then drop
        // while let Some(_) =  self.pop() {}
        self.len = 0;
    }
}

// slice trait implementation is done
// via Deref and DerefMut trait implementation

impl<T> Deref for NaiveVec<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        unsafe {
            std::slice::from_raw_parts(self.ptr(), self.len)
        }
    }
}

impl<T> DerefMut for NaiveVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr(), self.len)
        }
    }
}

pub struct Drain<'a, T: 'a> {
    vec: PhantomData<&'a mut NaiveVec<T>>,
    iter: RawValIter<T>,
}

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}