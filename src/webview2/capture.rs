use std::{
  cell::Cell,
  rc::Rc,
  sync::{atomic::AtomicBool, mpsc, Arc},
  thread,
};

use windows::{
  Foundation::AsyncActionCompletedHandler,
  System::DispatcherQueueController,
  Win32::{
    System::{Threading::GetCurrentThreadId, WinRT::RoInitialize},
    UI::WindowsAndMessaging::{
      DispatchMessageW, GetMessageW, PostQuitMessage, TranslateMessage, MSG,
    },
  },
};
use windows_capture::{
  capture::{GraphicsCaptureApiError, GraphicsCaptureApiHandler},
  frame::{Frame, FrameBuffer},
  graphics_capture_api::{GraphicsCaptureApi, InternalCaptureControl},
  settings::Settings,
};

use std::{mem, os::windows::prelude::AsRawHandle, sync::atomic, thread::JoinHandle};

use parking_lot::Mutex;
use windows::Win32::{
  Foundation::{HANDLE, LPARAM, WPARAM},
  System::{
    Threading::GetThreadId,
    WinRT::{
      CreateDispatcherQueueController, DispatcherQueueOptions, RoUninitialize, DQTAT_COM_NONE,
      DQTYPE_THREAD_CURRENT, RO_INIT_MULTITHREADED,
    },
  },
  UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT},
};

#[derive(Debug, Clone)]
pub(crate) struct OwnedFrame {
  pub data: Box<[u8]>,
  pub width: u32,
  pub height: u32,
}

impl<'a, 'b> TryFrom<&'a mut Frame<'b>> for OwnedFrame {
  fn try_from(value: &'a mut Frame<'b>) -> Result<Self, Self::Error> {
    Ok(OwnedFrame {
      data: Box::from(value.buffer()?.as_raw_nopadding_buffer()?.to_owned()),
      width: value.width(),
      height: value.height(),
    })
  }

  type Error = Box<dyn std::error::Error + Send + Sync>;
}

pub(crate) struct RecordingContext {
  frame: &'static std::sync::Mutex<Option<OwnedFrame>>,
}

impl RecordingContext {
  pub fn custom_start(
    settings: Settings<<RecordingContext as GraphicsCaptureApiHandler>::Flags>,
    controller: DispatcherQueueController,
  ) -> Result<(), GraphicsCaptureApiError<<RecordingContext as GraphicsCaptureApiHandler>::Error>>
  where
    Self: Send + 'static,
    <Self as GraphicsCaptureApiHandler>::Flags: Send,
  {
    let (halt_sender, halt_receiver) = mpsc::channel::<Arc<AtomicBool>>();
    let (callback_sender, callback_receiver) = mpsc::channel::<Arc<Mutex<Self>>>();

    let thread_handle = thread::spawn(
      move || -> Result<(), GraphicsCaptureApiError<Box<dyn std::error::Error + Send + Sync>>> {
        // Create a dispatcher queue for the current thread
        let options = DispatcherQueueOptions {
          dwSize: u32::try_from(mem::size_of::<DispatcherQueueOptions>()).unwrap(),
          threadType: DQTYPE_THREAD_CURRENT,
          apartmentType: DQTAT_COM_NONE,
        };
        let controller = unsafe {
          CreateDispatcherQueueController(options)
            .map_err(|_| GraphicsCaptureApiError::FailedToCreateDispatcherQueueController)?
        };

        // Get current thread ID
        let thread_id = unsafe { GetCurrentThreadId() };

        // Start capture
        let result = Arc::new(Mutex::new(None));
        let callback = Arc::new(Mutex::new(
          Self::new(settings.flags).map_err(GraphicsCaptureApiError::NewHandlerError)?,
        ));
        let mut capture = GraphicsCaptureApi::new(
          settings.item,
          callback.clone(),
          settings.cursor_capture,
          settings.draw_border,
          settings.color_format,
          thread_id,
          result.clone(),
        )
        .map_err(GraphicsCaptureApiError::GraphicsCaptureApiError)?;
        capture
          .start_capture()
          .map_err(GraphicsCaptureApiError::GraphicsCaptureApiError)?;

        // Send halt handle
        let halt_handle = capture.halt_handle();
        halt_sender.send(halt_handle).unwrap();

        // Send callback
        callback_sender.send(callback).unwrap();

        // Message loop
        let mut message = MSG::default();
        unsafe {
          while GetMessageW(&mut message, None, 0, 0).as_bool() {
            TranslateMessage(&message);
            DispatchMessageW(&message);
          }
        }

        // Shutdown dispatcher queue
        let async_action = controller
          .ShutdownQueueAsync()
          .map_err(|_| GraphicsCaptureApiError::FailedToShutdownDispatcherQueue)?;

        async_action
          .SetCompleted(&AsyncActionCompletedHandler::new(
            move |_, _| -> Result<(), windows::core::Error> {
              unsafe { PostQuitMessage(0) };
              Ok(())
            },
          ))
          .map_err(|_| GraphicsCaptureApiError::FailedToSetDispatcherQueueCompletedHandler)?;

        // Final message loop
        let mut message = MSG::default();
        unsafe {
          while GetMessageW(&mut message, None, 0, 0).as_bool() {
            TranslateMessage(&message);
            DispatchMessageW(&message);
          }
        }

        // Stop capture
        capture.stop_capture();

        // Check handler result
        if let Some(e) = result.lock().take() {
          return Err(GraphicsCaptureApiError::FrameHandlerError(e));
        }

        Ok(())
      },
    );
    println!("test");

    let Ok(halt_handle) = halt_receiver.recv() else {
      match thread_handle.join() {
        Ok(result) => return Err(result.err().unwrap()),
        Err(_) => {
          return Err(GraphicsCaptureApiError::FailedToJoinThread);
        }
      }
    };

    let Ok(callback) = callback_receiver.recv() else {
      match thread_handle.join() {
        Ok(result) => return Err(result.err().unwrap()),
        Err(_) => {
          return Err(GraphicsCaptureApiError::FailedToJoinThread);
        }
      }
    };
    println!("foo");

    // Ok(CaptureControl::new(thread_handle, halt_handle, callback))
    Ok(())

    /*
      // Get current thread ID
      let thread_id = unsafe { GetCurrentThreadId() };

      // Start capture
      let result = Arc::new(parking_lot::Mutex::new(None));
      let callback = Arc::new(parking_lot::Mutex::new(
        Self::new(settings.flags).map_err(GraphicsCaptureApiError::NewHandlerError)?,
      ));
      let mut capture = GraphicsCaptureApi::new(
        settings.item,
        callback,
        settings.cursor_capture,
        settings.draw_border,
        settings.color_format,
        thread_id,
        result.clone(),
      )
      .map_err(GraphicsCaptureApiError::GraphicsCaptureApiError)?;
      capture
        .start_capture()
        .map_err(GraphicsCaptureApiError::GraphicsCaptureApiError)?;

      // Message loop
      let mut message = MSG::default();
      unsafe {
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
          TranslateMessage(&message);
          DispatchMessageW(&message);
        }
      }

      // Shutdown dispatcher queue
      let async_action = controller
        .ShutdownQueueAsync()
        .map_err(|_| GraphicsCaptureApiError::FailedToShutdownDispatcherQueue)?;
      async_action
        .SetCompleted(&AsyncActionCompletedHandler::new(
          move |_, _| -> Result<(), windows::core::Error> {
            unsafe { PostQuitMessage(0) };
            Ok(())
          },
        ))
        .map_err(|_| GraphicsCaptureApiError::FailedToSetDispatcherQueueCompletedHandler)?;

      // Final message loop
      let mut message = MSG::default();
      unsafe {
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
          TranslateMessage(&message);
          DispatchMessageW(&message);
        }
      }

      // Stop capture
      capture.stop_capture();

      // Check handler result
      if let Some(e) = result.lock().take() {
        return Err(GraphicsCaptureApiError::FrameHandlerError(e));
      }

      Ok(())
    */
  }
}

impl GraphicsCaptureApiHandler for RecordingContext {
  // The type of flags used to get the values from the settings.
  type Flags = &'static std::sync::Mutex<Option<OwnedFrame>>;

  // The type of error that can occur during capture, the error will be returned from `CaptureControl` and `start` functions.
  type Error = Box<dyn std::error::Error + Send + Sync>;

  // Function that will be called to create the struct. The flags can be passed from settings.
  fn new(frame_storage: Self::Flags) -> Result<Self, Self::Error> {
    println!("Starting frame capture");

    Ok(Self {
      frame: frame_storage,
    })
  }

  // Called every time a new frame is available.
  fn on_frame_arrived(
    &mut self,
    frame: &mut Frame,
    _capture_control: InternalCaptureControl,
  ) -> Result<(), Self::Error> {
    println!("got frame");
    println!(
      "{:?}",
      &frame.buffer().unwrap().as_raw_nopadding_buffer().unwrap()[0..20]
    );
    *self.frame.lock().map_err(|_| "mutex poisoned")? = Some(frame.try_into()?);

    Ok(())
  }

  // Optional handler called when the capture item (usually a window) closes.
  fn on_closed(&mut self) -> Result<(), Self::Error> {
    println!("Capture Session Closed");

    Ok(())
  }
}
