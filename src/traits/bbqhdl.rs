use core::ops::Deref;
use std::sync::Arc;

use crate::queue::{ArcBBQueue, BBQueue};

use super::{coordination::Coord, notifier::Notifier, storage::Storage};

pub trait BbqHandle {
    type Target: Deref<Target = BBQueue<Self::Storage, Self::Coord, Self::Notifier>> + Clone;
    fn bbq_ref(&self) -> Self::Target;

    type Storage: Storage;
    type Coord: Coord;
    type Notifier: Notifier;
}

impl<S: Storage, C: Coord, N: Notifier> BbqHandle for &BBQueue<S, C, N> {
    type Target = Self;

    #[inline(always)]
    fn bbq_ref(&self) -> Self::Target {
        *self
    }

    type Storage = S;
    type Coord = C;
    type Notifier = N;
}

#[cfg(feature = "std")]
impl<S: Storage, C: Coord, N: Notifier> BbqHandle for ArcBBQueue<S, C, N> {
    type Target = Arc<BBQueue<S, C, N>>;

    #[inline(always)]
    fn bbq_ref(&self) -> Self::Target {
        self.0.clone()
    }

    type Storage = S;
    type Coord = C;
    type Notifier = N;
}

#[cfg(feature = "std")]
impl<S: Storage, C: Coord, N: Notifier> BbqHandle for Arc<BBQueue<S, C, N>> {
    type Target = Self;

    #[inline(always)]
    fn bbq_ref(&self) -> Self::Target {
        self.clone()
    }

    type Storage = S;
    type Coord = C;
    type Notifier = N;
}
