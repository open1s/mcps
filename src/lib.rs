// Copyright (c) { props["inceptionYear"] } { props["copyrightOwner"] }
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.
pub mod schema;
pub mod transport;
pub mod support;
pub mod config;
pub mod server;
pub mod client;

pub use support::definition::error::MCPError;


#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use iconfig;
    use iconfig::ApplicationConfig;
    use crate::config::transport_config::TransportConfig;

    #[test]
   fn test_load_application_config() {
      let config = iconfig::load().unwrap();

      let transport: TransportConfig = config.resolve_prefix("transport").unwrap();
      println!("{:?}", transport);

      let transport = config.resolve_prefix::<TransportConfig>("transport").unwrap();
      println!("{:?}", transport);
   }

    #[rioc::provider]
    #[provide(Arc<ApplicationConfig>, || Arc::new(iconfig::load().unwrap()))]
    struct InitProvider{
    }

//     #[test]
//    fn test_resolve_prefix() {
//        let init_provider = ConfigProvider::new();
//        let config = init_provider.resolve::<Arc<ApplicationConfig>>().unwrap();
//        println!("{:?}", config);
//    }
}