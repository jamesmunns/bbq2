use core::{
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::{
    queue::{ArcBBQueue, BBQueue},
    traits::{
        bbqhdl::BbqHandle,
        coordination::Coord,
        notifier::{
            typed::{AsyncNotifierTyped, ConstrFnMut, ConstrFut, Typed, TypedWrapper},
            AsyncNotifier, Notifier,
        },
        storage::Storage,
    },
};

impl<S: Storage, C: Coord, N: Notifier> BBQueue<S, C, N> {
    pub fn stream_producer(&self) -> StreamProducer<&Self> {
        StreamProducer {
            bbq: self.bbq_ref(),
        }
    }

    pub fn stream_consumer(&self) -> StreamConsumer<&Self> {
        StreamConsumer {
            bbq: self.bbq_ref(),
        }
    }
}

impl<S: Storage, C: Coord, N: Notifier> ArcBBQueue<S, C, N> {
    pub fn stream_producer(&self) -> StreamProducer<Self> {
        StreamProducer {
            bbq: self.bbq_ref(),
        }
    }

    pub fn stream_consumer(&self) -> StreamConsumer<Self> {
        StreamConsumer {
            bbq: self.bbq_ref(),
        }
    }
}

pub struct StreamProducer<Q: BbqHandle> {
    bbq: Q::Target,
}

impl<Q: BbqHandle> StreamProducer<Q> {
    pub fn grant_max_remaining(&self, max: usize) -> Result<StreamGrantW<Q>, ()> {
        let (ptr, cap) = self.bbq.sto.ptr_len();
        let (offset, len) = self.bbq.cor.grant_max_remaining(cap, max)?;
        let ptr = unsafe {
            let p = ptr.as_ptr().byte_add(offset);
            NonNull::new_unchecked(p)
        };
        Ok(StreamGrantW {
            bbq: self.bbq.clone(),
            ptr,
            len,
            to_commit: 0,
        })
    }

    pub fn grant_exact(&self, sz: usize) -> Result<StreamGrantW<Q>, ()> {
        let (ptr, cap) = self.bbq.sto.ptr_len();
        let offset = self.bbq.cor.grant_exact(cap, sz)?;
        let ptr = unsafe {
            let p = ptr.as_ptr().byte_add(offset);
            NonNull::new_unchecked(p)
        };
        Ok(StreamGrantW {
            bbq: self.bbq.clone(),
            ptr,
            len: sz,
            to_commit: 0,
        })
    }
}

impl<Q: BbqHandle<Notifier: AsyncNotifier>> StreamProducer<Q> {
    pub async fn wait_grant_max_remaining(&self, max: usize) -> StreamGrantW<Q> {
        self.bbq
            .not
            .wait_for_not_full(|| self.grant_max_remaining(max).ok())
            .await
    }

    pub async fn wait_grant_exact(&self, sz: usize) -> StreamGrantW<Q> {
        self.bbq
            .not
            .wait_for_not_full(|| self.grant_exact(sz).ok())
            .await
    }
}

impl<Q: BbqHandle> Typed for StreamProducer<Q> {}

impl<Q: BbqHandle<Notifier: AsyncNotifierTyped>> TypedWrapper<StreamProducer<Q>> {
    pub fn wait_grant_max_remaining(
        &self,
        max: usize,
    ) -> <Q::Notifier as ConstrFut>::NotFull<impl ConstrFnMut<Out = StreamGrantW<Q>> + '_> {
        AsyncNotifierTyped::wait_for_not_full(&self.bbq.not, move || {
            self.grant_max_remaining(max).ok()
        })
    }

    pub fn wait_grant_exact(
        &self,
        sz: usize,
    ) -> <Q::Notifier as ConstrFut>::NotFull<impl ConstrFnMut<Out = StreamGrantW<Q>> + '_> {
        AsyncNotifierTyped::wait_for_not_full(&self.bbq.not, move || self.grant_exact(sz).ok())
    }
}

pub struct StreamConsumer<Q: BbqHandle> {
    bbq: Q::Target,
}

impl<Q: BbqHandle> StreamConsumer<Q> {
    pub fn read(&self) -> Result<StreamGrantR<Q>, ()> {
        let (ptr, _cap) = self.bbq.sto.ptr_len();
        let (offset, len) = self.bbq.cor.read()?;
        let ptr = unsafe {
            let p = ptr.as_ptr().byte_add(offset);
            NonNull::new_unchecked(p)
        };
        Ok(StreamGrantR {
            bbq: self.bbq.clone(),
            ptr,
            len,
            to_release: 0,
        })
    }
}

impl<Q: BbqHandle<Notifier: AsyncNotifier>> StreamConsumer<Q> {
    pub async fn wait_read(&self) -> StreamGrantR<Q> {
        self.bbq.not.wait_for_not_empty(|| self.read().ok()).await
    }
}

impl<Q: BbqHandle> Typed for StreamConsumer<Q> {}

impl<Q: BbqHandle<Notifier: AsyncNotifierTyped>> TypedWrapper<StreamConsumer<Q>> {
    pub fn wait_read(
        &self,
    ) -> <Q::Notifier as ConstrFut>::NotEmpty<impl ConstrFnMut<Out = StreamGrantR<Q>>> {
        AsyncNotifierTyped::wait_for_not_empty(&self.bbq.not, move || self.read().ok())
    }
}

pub struct StreamGrantW<Q: BbqHandle> {
    bbq: Q::Target,
    ptr: NonNull<u8>,
    len: usize,
    to_commit: usize,
}

impl<Q: BbqHandle> Deref for StreamGrantW<Q> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<Q: BbqHandle> DerefMut for StreamGrantW<Q> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<Q: BbqHandle> Drop for StreamGrantW<Q> {
    fn drop(&mut self) {
        let StreamGrantW {
            bbq,
            ptr: _,
            len,
            to_commit,
        } = self;
        let (_, cap) = bbq.sto.ptr_len();
        let len = *len;
        let used = (*to_commit).min(len);
        bbq.cor.commit_inner(cap, len, used);
        if used != 0 {
            bbq.not.wake_one_consumer();
        }
    }
}

impl<Q: BbqHandle> StreamGrantW<Q> {
    pub fn commit(self, used: usize) {
        let (_, cap) = self.bbq.sto.ptr_len();
        let used = used.min(self.len);
        self.bbq.cor.commit_inner(cap, self.len, used);
        if used != 0 {
            self.bbq.not.wake_one_consumer();
        }
        core::mem::forget(self);
    }
}

pub struct StreamGrantR<Q: BbqHandle> {
    bbq: Q::Target,
    ptr: NonNull<u8>,
    len: usize,
    to_release: usize,
}

impl<Q: BbqHandle> Deref for StreamGrantR<Q> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<Q: BbqHandle> DerefMut for StreamGrantR<Q> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<Q: BbqHandle> Drop for StreamGrantR<Q> {
    fn drop(&mut self) {
        let StreamGrantR {
            bbq,
            ptr: _,
            len,
            to_release,
        } = self;
        let len = *len;
        let used = (*to_release).min(len);
        bbq.cor.release_inner(used);
        if used != 0 {
            bbq.not.wake_one_producer();
        }
    }
}

impl<Q: BbqHandle> StreamGrantR<Q> {
    pub fn release(self, used: usize) {
        let used = used.min(self.len);
        self.bbq.cor.release_inner(used);
        if used != 0 {
            self.bbq.not.wake_one_producer();
        }
        core::mem::forget(self);
    }
}
