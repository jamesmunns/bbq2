use const_init::ConstInit;
use core::{cell::UnsafeCell, mem::MaybeUninit, ptr::NonNull};

pub trait Storage {
    fn ptr_len(&self) -> (NonNull<u8>, usize);
}

pub trait ConstStorage: Storage + ConstInit {}

impl<T> ConstStorage for T where T: Storage + ConstInit {}

#[repr(transparent)]
pub struct Inline<const N: usize> {
    buf: UnsafeCell<MaybeUninit<[u8; N]>>,
}

unsafe impl<const N: usize> Sync for Inline<N> {}

impl<const N: usize> Inline<N> {
    pub const fn new() -> Self {
        Self {
            buf: UnsafeCell::new(MaybeUninit::zeroed()),
        }
    }
}

impl<const N: usize> Default for Inline<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Storage for Inline<N> {
    fn ptr_len(&self) -> (NonNull<u8>, usize) {
        if N == 0 {
            return (NonNull::dangling(), N);
        }
        let ptr: *mut MaybeUninit<[u8; N]> = self.buf.get();
        let ptr: *mut u8 = ptr.cast();
        // SAFETY: UnsafeCell and MaybeUninit are both repr transparent, cast is
        // sound to get to first byte element
        let nn_ptr = unsafe { NonNull::new_unchecked(ptr) };
        (nn_ptr, N)
    }
}

#[allow(clippy::declare_interior_mutable_const)]
impl<const N: usize> ConstInit for Inline<N> {
    const INIT: Self = Self::new();
}

impl<const N: usize> Storage for &'_ Inline<N> {
    fn ptr_len(&self) -> (NonNull<u8>, usize) {
        let len = N;

        let ptr: *mut MaybeUninit<[u8; N]> = self.buf.get();
        let ptr: *mut u8 = ptr.cast();
        let nn_ptr = unsafe { NonNull::new_unchecked(ptr) };

        (nn_ptr, len)
    }
}

#[cfg(feature = "std")]
pub struct BoxedSlice {
    buf: Box<[UnsafeCell<MaybeUninit<u8>>]>,
}

#[cfg(feature = "std")]
unsafe impl Sync for BoxedSlice {}

#[cfg(feature = "std")]
impl BoxedSlice {
    pub fn new(len: usize) -> Self {
        let buf: Box<[UnsafeCell<MaybeUninit<u8>>]> = {
            let mut v: Vec<UnsafeCell<MaybeUninit<u8>>> = Vec::with_capacity(len);
            // Fields are already MaybeUninit, so valid capacity is valid len
            unsafe {
                v.set_len(len);
            }
            // We can zero each field now
            v.iter_mut().for_each(|val| {
                *val = UnsafeCell::new(MaybeUninit::zeroed());
            });
            v.into_boxed_slice()
        };
        Self { buf }
    }
}

#[cfg(feature = "std")]
impl Storage for BoxedSlice {
    fn ptr_len(&self) -> (NonNull<u8>, usize) {
        let len = self.buf.len();

        let ptr: *const UnsafeCell<MaybeUninit<u8>> = self.buf.as_ptr();
        let ptr: *mut MaybeUninit<u8> = UnsafeCell::raw_get(ptr);
        let ptr: *mut u8 = ptr.cast();
        let nn_ptr = unsafe { NonNull::new_unchecked(ptr) };

        (nn_ptr, len)
    }
}

#[cfg(test)]
mod test {
    use super::{Inline, Storage};

    #[test]
    fn provenance_slice() {
        let sli = Inline::<64>::new();
        let sli = &sli;
        let (ptr, len) = <&Inline<64> as Storage>::ptr_len(&sli);

        // This test ensures that obtaining the pointer for ptr_len through a single
        // element is sound
        for i in 0..len {
            unsafe {
                ptr.as_ptr().add(i).write(i as u8);
            }
        }
    }
}
