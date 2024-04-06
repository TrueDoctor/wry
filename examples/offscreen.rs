// Copyright 2020-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

use tao::{
  event::{Event, WindowEvent},
  event_loop::{ControlFlow, EventLoop},
  window::WindowBuilder,
};
use wry::WebViewBuilder;

fn main() -> wry::Result<()> {
  //let event_loop = EventLoop::new();
  //let window = WindowBuilder::new().build(&event_loop).unwrap();

  #[cfg(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "ios",
    target_os = "android"
  ))]
  //let builder = WebViewBuilder::new(&window);
  let builder = WebViewBuilder::new_offscreen();

  #[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "ios",
    target_os = "android"
  )))]
  let builder = {
    use tao::platform::unix::WindowExtUnix;
    use wry::WebViewBuilderExtUnix;
    let vbox = window.default_vbox().unwrap();
    WebViewBuilder::new_gtk(vbox)
  };

  let webview = builder
    .with_url("http://tauri.app")
    .with_drag_drop_handler(|e| {
      match e {
        wry::DragDropEvent::Enter { paths, position } => {
          println!("DragEnter: {position:?} {paths:?} ")
        }
        wry::DragDropEvent::Over { position } => println!("DragOver: {position:?} "),
        wry::DragDropEvent::Drop { paths, position } => {
          println!("DragDrop: {position:?} {paths:?} ")
        }
        wry::DragDropEvent::Leave => println!("DragLeave"),
        _ => {}
      }

      true
    })
    .build()?;

  /*event_loop.run(move |event, _, control_flow| {
    *control_flow = ControlFlow::Wait;

    if let Event::WindowEvent {
      event: WindowEvent::CloseRequested,
      ..
    } = event
    {
      *control_flow = ControlFlow::Exit
    }
  });*/

  let mut last = 0;
  //println!("data: {:?}", webview.offscreen_data());
  loop {
    if let Ok(x) = webview.offscreen_data() {
      //dbg!("got_data", &x);
      if x[0] != last {
        last = dbg!(x[0]);
        // if last == 217 {
        dbg!(x.len());
        println!("{x:?}");
        // }
      }
    }
  }
  Ok(())
}
