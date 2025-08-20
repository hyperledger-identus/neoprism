use rustyscript::{Error, Module, Runtime, json_args};

pub mod did;
pub mod error;
pub mod prelude;

pub fn call_ts_hello(name: &str) -> Result<String, Error> {
    let module = Module::load("examples/hello.ts")?;
    let mut runtime = Runtime::new(Default::default())?;
    let handle = runtime.load_module(&module)?;
    let greeting: String = runtime.call_function(Some(&handle), "hello", json_args!(name))?;
    Ok(greeting)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_call_ts_hello() {
        let result = call_ts_hello("NeoPrism").unwrap();
        assert_eq!(result, "Hello, NeoPrism!");
    }
}
