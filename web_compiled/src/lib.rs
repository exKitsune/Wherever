#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

pub const JS: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/web/wherever_web.js"));
pub const WASM: &'static [u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/web/wherever_web_bg.wasm"));
pub const HTML: &'static [u8] = include_bytes!(concat!(env!("OUT_DIR"), "/web/index.html"));
