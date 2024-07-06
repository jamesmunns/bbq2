use core::{future::Future, marker::PhantomData, ops::{Deref, DerefMut}, pin::{self, Pin}, task::Poll};

use maitake_sync::{
    wait_cell::{Subscribe, Wait},
    wait_queue::{Wait as QWait},
    WaitCell, WaitQueue,
};

use super::{AsyncNotifier, Notifier};

pub struct MaiNotSpsc {
    not_empty: WaitCell,
    not_full: WaitCell,
}

impl MaiNotSpsc {
    pub fn new() -> Self {
        Self::INIT
    }
}

impl Default for MaiNotSpsc {
    fn default() -> Self {
        Self::new()
    }
}

impl Notifier for MaiNotSpsc {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = Self {
        not_empty: WaitCell::new(),
        not_full: WaitCell::new(),
    };

    fn wake_one_consumer(&self) {
        _ = self.not_empty.wake();
    }

    fn wake_one_producer(&self) {
        _ = self.not_full.wake();
    }
}

pub struct SubWrap<'a> {
    s: Subscribe<'a>,
}

impl<'a> Future for SubWrap<'a> {
    type Output = WaitWrap<'a>;

    fn poll(
        mut self: pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let pinned = pin::pin!(&mut self.s);
        pinned.poll(cx).map(|w| WaitWrap { w })
    }
}

pub struct WaitWrap<'a> {
    w: Wait<'a>,
}

impl<'a> Future for WaitWrap<'a> {
    type Output = ();

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let pinned = pin::pin!(&mut self.w);
        pinned.poll(cx).map(drop)
    }
}

impl AsyncNotifier for MaiNotSpsc {
    type NotEmptyRegisterFut<'a> = SubWrap<'a>;
    type NotFullRegisterFut<'a> = SubWrap<'a>;
    type NotEmptyWaiterFut<'a> = WaitWrap<'a>;
    type NotFullWaiterFut<'a> = WaitWrap<'a>;

    fn register_wait_not_empty(&self) -> Self::NotEmptyRegisterFut<'_> {
        SubWrap {
            s: self.not_empty.subscribe(),
        }
    }

    fn register_wait_not_full(&self) -> Self::NotFullRegisterFut<'_> {
        SubWrap {
            s: self.not_full.subscribe(),
        }
    }
}

// ---


pub struct MaiNotMpsc {
    not_empty: WaitCell,
    not_full: WaitQueue,
}

impl MaiNotMpsc {
    pub fn new() -> Self {
        Self::INIT
    }
}

impl Default for MaiNotMpsc {
    fn default() -> Self {
        Self::new()
    }
}

impl Notifier for MaiNotMpsc {
    #[allow(clippy::declare_interior_mutable_const)]
    const INIT: Self = Self {
        not_empty: WaitCell::new(),
        not_full: WaitQueue::new(),
    };

    fn wake_one_consumer(&self) {
        _ = self.not_empty.wake();
    }

    fn wake_one_producer(&self) {
        self.not_full.wake();
    }
}

pub struct QueueWrap<'a> {
    wq: &'a WaitQueue,
}

impl<'a> Future for QueueWrap<'a> {
    type Output = QWaitWrap<'a>;

    fn poll(
        self: pin::Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        Poll::Ready(QWaitWrap {
            w: self.wq.wait(),
        })
    }
}

// impl<'a> Future for SubWrap<'a> {
//     type Output = WaitWrap<'a>;

//     fn poll(
//         mut self: pin::Pin<&mut Self>,
//         cx: &mut core::task::Context<'_>,
//     ) -> core::task::Poll<Self::Output> {
//         let pinned = pin::pin!(&mut self.s);
//         pinned.poll(cx).map(|w| WaitWrap { w })
//     }
// }

#[pin_project::pin_project]
pub struct QWaitWrap<'a> {
    #[pin]
    w: QWait<'a>,
}

impl<'a> Deref for QWaitWrap<'a> {
    type Target = QWait<'a>;

    fn deref(&self) -> &Self::Target {
        &self.w
    }
}

impl<'a> DerefMut for QWaitWrap<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.w
    }
}

impl<'a> Future for QWaitWrap<'a> {
    type Output = ();

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = self.as_mut().project();
        this.w.poll(cx).map(drop)
    }
}

impl AsyncNotifier for MaiNotMpsc {
    type NotEmptyRegisterFut<'a> = SubWrap<'a>;
    type NotFullRegisterFut<'a> = QueueWrap<'a>;

    type NotEmptyWaiterFut<'a> = WaitWrap<'a>;
    type NotFullWaiterFut<'a> = QWaitWrap<'a>;

    fn register_wait_not_empty(&self) -> Self::NotEmptyRegisterFut<'_> {
        SubWrap {
            s: self.not_empty.subscribe(),
        }
    }

    fn register_wait_not_full(&self) -> Self::NotFullRegisterFut<'_> {
        QueueWrap {
            wq: &self.not_full,
        }
    }
}
