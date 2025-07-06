use core::{marker::PhantomData, ops::Deref};

use crate::{prod_cons::{framed::{FramedConsumer, FramedProducer, LenHeader}, stream::{StreamConsumer, StreamProducer}}, queue::BBQueue};

use super::{coordination::Coord, notifier::Notifier, storage::Storage};

pub trait BbqHandle: Sized {
    type Target: Deref<Target = BBQueue<Self::Storage, Self::Coord, Self::Notifier>> + Clone + Sized;
    type Storage: Storage;
    type Coord: Coord;
    type Notifier: Notifier;
    fn bbq_ref(&self) -> Self::Target;

    fn stream_producer(&self) -> StreamProducer<Self> {
        StreamProducer {
            bbq: self.bbq_ref(),
        }
    }

    fn stream_consumer(&self) -> StreamConsumer<Self> {
        StreamConsumer {
            bbq: self.bbq_ref(),
        }
    }

    fn framed_producer<H: LenHeader>(&self) -> FramedProducer<Self, H> {
        FramedProducer {
            bbq: self.bbq_ref(),
            pd: PhantomData,
        }
    }

    fn framed_consumer<H: LenHeader>(&self) -> FramedConsumer<Self, H> {
        FramedConsumer {
            bbq: self.bbq_ref(),
            pd: PhantomData,
        }
    }
}

impl<S: Storage, C: Coord, N: Notifier> BbqHandle for &'_ BBQueue<S, C, N> {
    type Target = Self;
    type Storage = S;
    type Coord = C;
    type Notifier = N;

    #[inline(always)]
    fn bbq_ref(&self) -> Self::Target {
        *self
    }
}

#[cfg(feature = "std")]
impl<S: Storage, C: Coord, N: Notifier> BbqHandle for std::sync::Arc<BBQueue<S, C, N>> {
    type Target = Self;
    type Storage = S;
    type Coord = C;
    type Notifier = N;

    #[inline(always)]
    fn bbq_ref(&self) -> Self::Target {
        self.clone()
    }
}
