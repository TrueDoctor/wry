// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use dpi::{LogicalPosition, LogicalSize};
use tao::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};
use wry::{Rect, WebViewBuilder};

fn main() -> Result<(), ()> {
  use tao::platform::unix::WindowExtUnix;
  use wry::WebViewBuilderExtUnix;

  gtk::init().unwrap();

  // we need to ignore this error here otherwise it will be catched by winit and will be
  // make the example crash
  let builder = WebViewBuilder::new_offscreen();

  let webview = builder
    .with_bounds(Rect {
      position: LogicalPosition::new(0, 0).into(),
      size: LogicalSize::new(20, 20).into(),
    })
    .with_transparent(true)
    .with_html(
      r#"<html>
          <body style="background-color:rgba(0,255,0,0.9);"></body>
        </html>"#,
    )
    // .with_url("http://tauri.app")
    .build()
    .unwrap();

  let mut last = 0;
  loop {
    if let Ok(x) = webview.offscreen_data() {
      if x[0] != last {
        last = dbg!(x[0]);
        // if last == 217 {
        dbg!(x.len());
        println!("{x:?}");
        // }
      }
    }
    gtk::main_iteration_do(false);
  }
}
