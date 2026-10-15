//! Port of `HeadlessVncPlatformExtensions.cs`: starts an application on the
//! headless platform with a VNC server that shows its main window.
//!
//! Upstream accepts the connections in an asynchronous loop on the UI
//! thread. Here a thread of the server waits for them and hands each to
//! the UI thread, which creates its session as upstream does; the loop
//! starts when the application starts, and the listener is bound before
//! that, as upstream.

use crate::ferro_vnc_logger::FerroVncLogger;
use crate::headless_vnc_framebuffer_source::HeadlessVncFramebufferSource;
use crate::remote_viewing::logging::ILogger;
use crate::remote_viewing::vnc::server::{VncServerSession, VncServerSessionOptions};
use crate::remote_viewing::vnc::{
    AuthenticationMethod, IVncFramebufferSource, VncPasswordChallenge,
};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::platform::PixelFormats;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_controls::{AppBuilder, Application, ShutdownMode};
use ferroui_headless::{FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions};
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// The extensions of the application builder that start an application on
/// the headless platform with a VNC server.
pub trait HeadlessVncPlatformExtensions {
    /// Starts the application with the headless VNC platform without
    /// password.
    ///
    /// `host` is the IP address the VNC server binds to (the loopback
    /// address if `None` or empty), `port` its port, `args` the start
    /// arguments of the application and `shutdown_mode` the shutdown mode
    /// of its lifetime (`OnLastWindowClose` in the original when it is not
    /// given).
    ///
    /// Returns the exit code of the application.
    ///
    /// # Panics
    /// When the address cannot be parsed or bound (the exceptions of the
    /// original).
    fn start_with_headless_vnc_platform(
        &self,
        host: Option<&str>,
        port: u16,
        args: &[String],
        shutdown_mode: ShutdownMode,
    ) -> i32;

    /// Starts the application with the headless VNC platform with password:
    /// `password` is the password of the VNC authentication of a connection
    /// (no authentication if it is `None` or blank).
    ///
    /// # Panics
    /// When the address cannot be parsed or bound (the exceptions of the
    /// original).
    fn start_with_headless_vnc_platform_with_password(
        &self,
        host: Option<&str>,
        port: u16,
        password: Option<&str>,
        args: &[String],
        shutdown_mode: ShutdownMode,
    ) -> i32;
}

impl HeadlessVncPlatformExtensions for AppBuilder {
    fn start_with_headless_vnc_platform(
        &self,
        host: Option<&str>,
        port: u16,
        args: &[String],
        shutdown_mode: ShutdownMode,
    ) -> i32 {
        self.start_with_headless_vnc_platform_with_password(host, port, None, args, shutdown_mode)
    }

    fn start_with_headless_vnc_platform_with_password(
        &self,
        host: Option<&str>,
        port: u16,
        password: Option<&str>,
        args: &[String],
        shutdown_mode: ShutdownMode,
    ) -> i32 {
        let vnc_logger: Arc<dyn ILogger> = Arc::new(FerroVncLogger);
        let address = match host {
            None | Some("") => IpAddr::V4(Ipv4Addr::LOCALHOST),
            Some(host) => host
                .parse::<IpAddr>()
                .unwrap_or_else(|error| panic!("{host}: {error}")),
        };
        let tcp_server = TcpListener::bind((address, port))
            .unwrap_or_else(|error| panic!("{address}:{port}: {error}"));
        let password = password
            .filter(|password| !password.trim().is_empty())
            .map(str::to_owned);

        self.use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            frame_buffer_format: PixelFormats::BGRA8888,
            ..FerroHeadlessPlatformOptions::default()
        })
        .after_application_setup(move |builder| {
            let lifetime = builder
                .instance()
                .and_then(|instance| instance.application_lifetime());
            let Some(lt) = lifetime
                .as_ref()
                .and_then(|lt| lt.as_controlled_application_lifetime())
            else {
                panic!("The application lifetime is not a classic desktop style lifetime.");
            };

            let tcp_server = tcp_server
                .try_clone()
                .expect("the listener of the VNC server is shared");
            let vnc_logger = vnc_logger.clone();
            let password = password.clone();
            // The subscription lasts as long as the lifetime: the handle is not needed.
            drop(lt.startup(Rc::new(move |_| {
                let tcp_server = tcp_server
                    .try_clone()
                    .expect("the listener of the VNC server is shared");
                let vnc_logger = vnc_logger.clone();
                let password = password.clone();
                thread::Builder::new()
                    .name("VNC server".to_owned())
                    .spawn(move || accept_clients(tcp_server, vnc_logger, password))
                    .expect("the thread of the VNC server starts");
            })));
        })
        .start_with_classic_desktop_lifetime_and_shutdown_mode(args, shutdown_mode)
    }
}

/// The loop of the server: accepts a client, hands it to the UI thread,
/// waits 100 milliseconds, and again.
fn accept_clients(tcp_server: TcpListener, vnc_logger: Arc<dyn ILogger>, password: Option<String>) {
    loop {
        match tcp_server.accept() {
            Ok((client, _)) => {
                let vnc_logger = vnc_logger.clone();
                let password = password.clone();
                Dispatcher::ui_thread().post(
                    move || {
                        if let Err(error) = start_session(client, vnc_logger, password) {
                            log_error(&error);
                        }
                    },
                    DispatcherPriority::NORMAL,
                );
            }
            Err(error) => log_error(&error.to_string()),
        }

        thread::sleep(Duration::from_millis(100));
    }
}

/// Creates the session of a client, on the UI thread: its framebuffer
/// source shows the main window.
fn start_session(
    client: TcpStream,
    vnc_logger: Arc<dyn ILogger>,
    password: Option<String>,
) -> Result<(), String> {
    let lt = Application::current()
        .and_then(|application| application.application_lifetime())
        .ok_or_else(|| "The application has no lifetime".to_owned())?;
    let lt = lt
        .as_classic_desktop_style_application_lifetime()
        .ok_or_else(|| {
            "The application lifetime is not a classic desktop style lifetime".to_owned()
        })?;

    let options = VncServerSessionOptions {
        authentication_method: if password.is_none() {
            AuthenticationMethod::None
        } else {
            AuthenticationMethod::Password
        },
    };
    let session =
        VncServerSession::new_with(Arc::new(VncPasswordChallenge::new()), Some(vnc_logger));
    if let Some(password) = password {
        let password: Vec<char> = password.chars().collect();
        session.password_provided().add(move |_, e| {
            e.accept_chars(&password);
        });
    }

    let main_window = lt
        .main_window()
        .ok_or_else(|| "MainWindow wasn't initialized".to_owned())?;
    let source: Arc<dyn IVncFramebufferSource> =
        HeadlessVncFramebufferSource::new(&session, main_window);
    session.set_framebuffer_source(Some(source));
    session.connect(Arc::new(client), Some(options));
    Ok(())
}

fn log_error(error: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VNC_PLATFORM) {
        logger.log(None, &format!("Error accepting client:{error}"));
    }
}
