use crate::traits::notifier::{AsyncNotifier, Notifier};
use core::{future::Future, ops::Deref};

pub trait AsyncNotifierTyped: Notifier {
    type FutNotEmpty<F, T>: Future<Output = T>;
    type FutNotFull<F, T>: Future<Output = T>;

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

impl<T> Typed for T {}

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
