use std::borrow::{Borrow, BorrowMut};
use std::io::{self, IoSlice, Write};
use std::iter::FromIterator;
use std::num::NonZeroUsize;
use std::ops::{Deref, DerefMut, Index, RangeBounds};
use std::slice::SliceIndex;
use std::vec::Splice;

macro_rules! copy_fn {
    (@single $v:vis fn $fname:ident(&mut self $(, $arg:ident: $t:ty)*) $(-> $ret:ty)?;) => {
        #[inline]
        $v fn $fname(&mut self $(, $arg: $t)*) $(-> $ret)? {
            Vec::$fname(&mut self.inner, $($arg),*)
        }
    };
    (@single $v:vis fn $fname:ident(&self $(, $arg:ident: $t:ty)*) $(-> $ret:ty)?;) => {
        #[inline]
        $v fn $fname(&self $(, $arg: $t),*) $(-> $ret)? {
            Vec::$fname(&self.inner, $($arg),*)
        }
    };
    (@single $v:vis fn $fname:ident(self $(, $arg:ident: $t:ty)*) $(-> $ret:ty)?;) => {
        #[inline]
        $v fn $fname(self $(, $arg: $t)*) $(-> $ret)? {
            Vec::$fname(self.inner, $($arg),*)
        }
    };
    (@single $v:vis fn $fname:ident($($arg:ident: $t:ty),*) $(-> $ret:ty)?;) => {
        #[inline]
        $v fn $fname($($arg: $t),*) $(-> $ret)? {
            Vec::$fname($($arg),*)
        }
    };

    ($($v:vis fn $fname:ident $t:tt $(-> $ret:ty)?;)*) => {
        $(
            copy_fn!(@single $v fn $fname $t $(-> $ret)?;);
        )*
    };
}

/// Like [`Vec<T>`](https://doc.rust-lang.org/std/vec/struct.Vec.html) but guaranteed to have at least one element.
///
/// Undocumented functions work exactly like their `Vec` counterpart
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct NonEmtpyVec<T> {
    inner: Vec<T>,
}

impl<T> Deref for NonEmtpyVec<T> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        &self.inner[..]
    }
}

impl<T> DerefMut for NonEmtpyVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        &mut self.inner[..]
    }
}

impl<T> NonEmtpyVec<T> {
    /// Constructs a new `NonEmptyVec<T>` from a single element
    #[inline]
    pub fn new(val: T) -> Self {
        Self { inner: vec![val] }
    }

    #[inline]
    pub fn with_capacity(capacity: NonZeroUsize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity.get()),
        }
    }

    #[inline]
    pub unsafe fn from_raw_parts(
        ptr: *mut T,
        length: NonZeroUsize,
        capacity: NonZeroUsize,
    ) -> Self {
        Self {
            inner: Vec::from_raw_parts(ptr, length.get(), capacity.get()),
        }
    }

    /// Constructs a new `NonEmptyVec<T>` from a `Vec` (returns `None` iff the `Vec` is empty)
    #[inline]
    pub fn from_vec(v: Vec<T>) -> Option<Self> {
        if v.is_empty() {
            None
        } else {
            Some(Self { inner: v })
        }
    }

    #[inline]
    pub fn capacity(&self) -> NonZeroUsize {
        // this is ok since it relies on the safety guarantee that there is always at least one element
        unsafe { NonZeroUsize::new_unchecked(self.inner.capacity()) }
    }

    copy_fn! {
        pub fn reserve(&mut self, additional: usize);
        pub fn reserve_exact(&mut self, additional: usize);
        pub fn shrink_to_fit(&mut self);
        pub fn into_boxed_slice(self) -> Box<[T]>;
    }

    #[inline]
    pub fn truncate(&mut self, len: NonZeroUsize) {
        self.inner.truncate(len.get())
    }

    copy_fn! {
        pub fn as_slice(&self) -> &[T];
        pub fn as_mut_slice(&mut self) -> &mut [T];
        pub fn as_ptr(&self) -> *const T;
        pub fn as_mut_ptr(&mut self) -> *mut T;
    }

    /// Works like [`set_len`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.set_len) with one slight change
    ///
    /// # Safety
    /// This is (additionally to the constraints of [`set_len`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.set_len)) only safe when `new_len > 0`
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        self.inner.set_len(new_len)
    }

    /// Works like [`swap_remove`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.swap_remove) with one slight change
    ///
    /// # Panics
    /// Panics if `self.len() == 1`
    #[inline]
    pub fn swap_remove(&mut self, index: usize) -> T {
        assert!(
            self.len() >= 1,
            "tried to remove the last item of NonEmptyVec"
        );
        self.inner.swap_remove(index)
    }

    copy_fn! {
        pub fn insert(&mut self, index: usize, element: T);
    }

    /// Works like [`remove`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.remove) with one slight change
    ///
    /// # Panics
    /// Panics if `self.len() == 1`
    #[inline]
    pub fn remove(&mut self, index: usize) -> T {
        assert!(
            self.len() >= 1,
            "tried to remove the last item of NonEmptyVec"
        );
        self.inner.remove(index)
    }

    /// Works like [`retain`](https://doc.rust-lang.org/std/vec/struct.Vec.html#method.retain) with one slight change
    ///
    /// # Panics
    /// Panics if at the end there are no more items left
    #[inline]
    pub fn retain<F>(&mut self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        self.inner.retain(f);
        assert!(
            !self.inner.is_empty(),
            "no items left after NonEmptyVec::retain"
        )
    }

    #[inline]
    pub fn dedup_by_key<F, K>(&mut self, key: F)
    where
        F: FnMut(&mut T) -> K,
        K: PartialEq<K>,
    {
        self.inner.dedup_by_key(key);
    }

    #[inline]
    pub fn dedup_by<F>(&mut self, same_bucket: F)
    where
        F: FnMut(&mut T, &mut T) -> bool,
    {
        self.inner.dedup_by(same_bucket)
    }

    copy_fn!(pub fn push(&mut self, value: T););

    /// Will not pop the last item, instead returns `None`
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.inner.len() < 2 {
            None
        } else {
            self.inner.pop()
        }
    }

    #[inline]
    pub fn append(&mut self, other: &mut Self) {
        self.inner.append(&mut other.inner)
    }

    #[inline]
    pub fn append_vec(&mut self, other: &mut Vec<T>) {
        self.inner.append(other)
    }

    copy_fn!(pub fn len(&self) -> usize;);

    #[inline]
    pub fn split_off(&mut self, at: NonZeroUsize) -> Vec<T> {
        self.inner.split_off(at.get())
    }

    #[inline]
    pub fn resize_with<F>(&mut self, new_len: NonZeroUsize, f: F)
    where
        F: FnMut() -> T,
    {
        self.inner.resize_with(new_len.get(), f)
    }

    /// Creates a new `NonEmptyVec` by applying `f` to every element of `self`
    #[inline]
    pub fn map<F: FnMut(T) -> U, U>(self, f: F) -> NonEmtpyVec<U> {
        NonEmtpyVec {
            inner: self.inner.into_iter().map(f).collect(),
        }
    }

    /// Applies `f` to every element of `self`
    #[inline]
    pub fn map_in_place<F: FnMut(T) -> T>(&mut self, f: F) {
        unsafe {
            std::ptr::write(
                &mut self.inner,
                std::ptr::read(&self.inner).into_iter().map(f).collect(),
            )
        }
    }
}

impl<T> NonEmtpyVec<T>
where
    T: Clone,
{
    #[inline]
    pub fn resize(&mut self, new_len: NonZeroUsize, value: T) {
        self.inner.resize(new_len.get(), value)
    }

    copy_fn!(pub fn extend_from_slice(&mut self, other: &[T]););
}

impl<T> NonEmtpyVec<T>
where
    T: PartialEq<T>,
{
    copy_fn!(pub fn dedup(&mut self););
}

impl<T> NonEmtpyVec<T> {
    #[inline]
    pub fn splice<R, I>(
        &mut self,
        range: R,
        replace_with: I,
    ) -> Splice<<I as IntoIterator>::IntoIter>
    where
        I: IntoIterator<Item = T>,
        R: RangeBounds<usize>,
    {
        self.inner.splice(range, replace_with)
    }
}

impl<T> AsRef<[T]> for NonEmtpyVec<T> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        &*self
    }
}

impl<T> AsRef<Vec<T>> for NonEmtpyVec<T> {
    #[inline]
    fn as_ref(&self) -> &Vec<T> {
        &self.inner
    }
}

impl<T> AsRef<NonEmtpyVec<T>> for NonEmtpyVec<T> {
    #[inline]
    fn as_ref(&self) -> &NonEmtpyVec<T> {
        self
    }
}

impl<T> AsMut<[T]> for NonEmtpyVec<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        &mut *self
    }
}

impl<T> AsMut<Vec<T>> for NonEmtpyVec<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut Vec<T> {
        &mut self.inner
    }
}

impl<T> AsMut<NonEmtpyVec<T>> for NonEmtpyVec<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut NonEmtpyVec<T> {
        self
    }
}

impl<T> Borrow<[T]> for NonEmtpyVec<T> {
    #[inline]
    fn borrow(&self) -> &[T] {
        &*self
    }
}

impl<T> BorrowMut<[T]> for NonEmtpyVec<T> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [T] {
        &mut *self
    }
}

impl<T: Default> Default for NonEmtpyVec<T> {
    #[inline]
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<'a, T> Extend<&'a T> for NonEmtpyVec<T>
where
    T: 'a + Copy,
{
    #[inline]
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T> Extend<T> for NonEmtpyVec<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.inner.extend(iter)
    }
}

impl<T> IntoIterator for NonEmtpyVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a NonEmtpyVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut NonEmtpyVec<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}

impl<T, I> Index<I> for NonEmtpyVec<T>
where
    I: SliceIndex<[T]>,
{
    type Output = I::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.inner.index(index)
    }
}

impl Write for NonEmtpyVec<u8> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        self.inner.write_vectored(bufs)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.inner.write_all(buf)
    }
}

pub trait TryFromIterator<T>: Sized {
    fn try_from_iter<I: Iterator<Item = T>>(iter: I) -> Option<Self>;
}

impl<T> TryFromIterator<T> for NonEmtpyVec<T> {
    fn try_from_iter<I: Iterator<Item = T>>(iter: I) -> Option<Self> {
        let v = Vec::from_iter(iter);
        NonEmtpyVec::from_vec(v)
    }
}

pub trait TryCollect: Iterator + Sized {
    fn try_collect<E: TryFromIterator<Self::Item>>(self) -> Option<E> {
        E::try_from_iter(self)
    }
}

impl<I: Iterator> TryCollect for I {}
