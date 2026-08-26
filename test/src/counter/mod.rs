use std::convert::Infallible;

#[anycall::service]
pub trait Counter {
    fn sum(&self, a: i32, b: i32) -> i32;
    async fn sum_async(&self, a: i32, b: i32) -> i32;
}

#[cfg(not(target_family = "wasm"))]
#[cfg(test)]
mod native_tests;

#[cfg(all(test, target_family = "wasm"))]
mod wasm_tests;

struct CounterNative;
impl CounterAsyncClient for CounterNative {
    type Err = Infallible;
    async fn sum(&self, a: i32, b: i32) -> Result<i32, Infallible> {
        Ok(a + b)
    }
    async fn sum_async(&self, a: i32, b: i32) -> Result<i32, Infallible> {
        Ok(a + b)
    }
}

pub struct CounterServerImpl;
impl CounterAsyncService for CounterServerImpl {
    type Ctx = ();
    async fn sum(&self, _ctx: (), a: i32, b: i32) -> i32 {
        a + b
    }

    async fn sum_async(&self, _ctx: (), a: i32, b: i32) -> i32 {
        a + b
    }
}
