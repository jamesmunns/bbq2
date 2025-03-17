use crate::traits::{
    bbqhdl::BbqHandle,
    coordination::Coord,
    notifier::{AsyncNotifier, Notifier},
    storage::Storage,
};
use core::{future::Future, ops::Deref};

pub trait AsyncNotifierTyped: Notifier {
    type FutNotEmpty<F: FnMut() -> Option<T>, T>: Future<Output = T>;
    type FutNotFull<F: FnMut() -> Option<T>, T>: Future<Output = T>;

    fn wait_for_not_empty<T, F: FnMut() -> Option<T>>(&self, f: F) -> Self::FutNotEmpty<F, T>;
    fn wait_for_not_full<T, F: FnMut() -> Option<T>>(&self, f: F) -> Self::FutNotFull<F, T>;
}

impl<N> AsyncNotifier for N
where
    N: AsyncNotifierTyped,
{
    async fn wait_for_not_empty<T, F: FnMut() -> Option<T>>(&self, f: F) -> T {
        self.wait_for_not_empty(f).await
    }

    async fn wait_for_not_full<T, F: FnMut() -> Option<T>>(&self, f: F) -> T {
        self.wait_for_not_full(f).await
    }
}

pub trait Typed: Sized {
    fn typed(self) -> TypedWrapper<Self> {
        TypedWrapper(self)
    }
}

pub struct TypedWrapper<T>(T);

impl<T> Deref for TypedWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub trait ConstrFnMut<'a>: FnMut() -> Option<Self::Out> {
    type Out;
}

impl<F, Out> ConstrFnMut<'_> for F
where
    F: FnMut() -> Option<Out>,
{
    type Out = Out;
}

#[allow(private_bounds)]
pub trait BbqSync<Q, S, C, N>
where
    Self: Imply<S, Is: Storage>,
    Self: Imply<C, Is: Coord>,
    Self: Imply<N, Is: Notifier>,
    Self: Imply<Q, Is: BbqHandle<S, C, N>>,
{
}

impl<T, Q, S, C, N> BbqSync<Q, S, C, N> for T
where
    Self: Imply<S, Is: Storage>,
    Self: Imply<C, Is: Coord>,
    Self: Imply<N, Is: Notifier>,
    Self: Imply<Q, Is: BbqHandle<S, C, N>>,
{
}

pub(crate) trait Imply<T>: ImplyInner<T, Is = T> {}

impl<S, T> Imply<T> for S {}

pub(crate) trait ImplyInner<T> {
    type Is;
}

impl<S, T> ImplyInner<T> for S {
    type Is = T;
}

pub trait ConstrFut<'a>: AsyncNotifierTyped {
    type NotFull<F: ConstrFnMut<'a>>;
    type NotEmpty<F: ConstrFnMut<'a>>;
}

impl<'a, N> ConstrFut<'a> for N
where
    N: AsyncNotifierTyped,
{
    type NotFull<F: ConstrFnMut<'a>> = <N as AsyncNotifierTyped>::FutNotFull<F, F::Out>;
    type NotEmpty<F: ConstrFnMut<'a>> = <N as AsyncNotifierTyped>::FutNotEmpty<F, F::Out>;
}
