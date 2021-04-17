use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

#[derive(Debug)]
pub struct PeekRef<'a, I> {
    container: NonNull<Option<Option<I>>>,
    inner: NonNull<I>,
    /// make sure we have the correct variance and
    /// the only one with access to the original reference
    /// while we are alive
    mut_ref: PhantomData<&'a mut I>,
}

impl<'a, I> Deref for PeekRef<'a, I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'a, I> DerefMut for PeekRef<'a, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<'a, I> PeekRef<'a, I> {
    pub fn new(option: &'a mut Option<Option<I>>) -> Option<PeekRef<'a, I>> {
        let option_ref = NonNull::from(&*option);
        if let Some(inner) = option.as_mut().and_then(|inner| inner.as_mut()) {
            Some(PeekRef {
                container: option_ref,
                inner: NonNull::from(inner),
                mut_ref: PhantomData::<&'a mut I>,
            })
        } else {
            None
        }
    }

    pub fn take(mut self) -> I {
        unsafe {
            // Safety:
            // This type is constructed from mutable references to Options that contain the Some variant
            // This types interface makes sure that the inner pointer stays valid until this value is
            // dropped or take is called
            //
            // As we consume self it is safe to invalidate inner as we don't use it here
            // and drop our self at the end of the function
            self.container.as_mut()
        }
        .take()
        .flatten()
        .expect("Constructed only for Some variant containing Some variant")
    }

    fn as_mut(&mut self) -> &mut I {
        unsafe {
            // Safety:
            // This type is constructed from mutable references to Options that contain the Some variant
            // This types interface makes sure that the inner pointer stays valid until this value is
            // dropped or take is called
            //
            // As we have a mutable reference to self we can't have given out another reference
            // currently
            self.inner.as_mut()
        }
    }

    fn as_ref(&self) -> &I {
        unsafe {
            // Safety:
            // This type is constructed from mutable references to Options that contain the Some variant
            // This types interface makes sure that the inner pointer stays valid until this value is
            // dropped or take is called
            //
            // As we have a reference to self we can't have given out a mutable reference
            // currently
            self.inner.as_ref()
        }
    }
}

/// A version of the standard libraries [`Peekable`](std::iter::Peekable) that lets one restore/replace/clear the peek element
#[derive(Debug, Clone)]
pub struct Peeker<I, IT> {
    iter: I,
    peek_window: Option<Option<IT>>,
}

impl<I, IT> Peeker<I, IT>
where
    I: Iterator<Item = IT>,
{
    /// Create a new `Peeker`
    pub fn new(inner: I) -> Self {
        Self {
            iter: inner,
            peek_window: None,
        }
    }

    /// Take a peek at the element that will be returned from the next next call
    pub fn peek(&mut self) -> Option<&IT> {
        self.peek_ref_mut().as_ref()
    }

    /// Take a mutable peek at the element that will be returned from the next next call
    /// Changing the value behind the reference will change the next element
    pub fn peek_mut(&mut self) -> Option<&mut IT> {
        self.peek_ref_mut().as_mut()
    }

    pub fn peek_ref(&mut self) -> Option<PeekRef<'_, IT>> {
        self.peek_ref_mut();
        PeekRef::new(&mut self.peek_window)
    }

    /// Make sure the peek slot is filled and return a mutable reference to the inner option
    fn peek_ref_mut(&mut self) -> &mut Option<IT> {
        let iter = &mut self.iter;
        self.peek_window.get_or_insert_with(|| iter.next())
    }

    /// Set a peek window if there currently is none
    ///
    /// # Panics
    /// If there is a window held as peek
    pub fn restore_peek(&mut self, window: IT) {
        if let None = self.peek_window.take().flatten() {
            self.peek_window = Some(Some(window))
        } else {
            panic!("Restoring over existing peek window!")
        }
    }
}

impl<I, IT> Iterator for Peeker<I, IT>
where
    I: Iterator<Item = IT>,
{
    type Item = IT;

    fn next(&mut self) -> Option<Self::Item> {
        self.peek_window.take().unwrap_or_else(|| self.iter.next())
    }
}
