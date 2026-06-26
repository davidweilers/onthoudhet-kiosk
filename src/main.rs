use iced::time::milliseconds;
use ::image as img;
use ::image::ColorType;
use iced::widget::{button, center_y, column, container, image, row, text, text_input};
use iced::window::screenshot::{self, Screenshot};
use iced::{Element, Subscription, Task, time, window};
use iced_webview::{Action, PageType, WebView};
use std::time::Duration;

type Engine = iced_webview::Litehtml; // or Blitz, Servo, Cef

#[derive(Debug, Clone)]
enum Message {
    WebView(Action),
    ViewCreated,
    Screenshot,
    Screenshotted(Screenshot),
    Png,
    PngSaved(Result<String, PngError>),
    Scp,
    Tick,
}

enum State {
    Idle,
    Ticking { last_tick: std::time::Instant },
}

struct App {
    webview: WebView<Engine, Message>,
    ready: bool,
    screenshot: Option<(Screenshot, image::Handle)>,
    saved_png_path: Option<Result<String, PngError>>,
    png_saving: bool,
    scp_sending: bool,
    state: State,
    tick_duration: std::time::Duration,
    tick_count: u32,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let webview = WebView::new()
            .on_create_view(Message::ViewCreated)
            .on_action(Message::WebView);
        (
            Self {
                webview,
                ready: false,
                screenshot: None,
                saved_png_path: None,
                png_saving: false,
                scp_sending: false,
                state: State::Idle,
                tick_duration: std::time::Duration::from_millis(100),
                tick_count: 0,
            },
            Task::done(Message::WebView(Action::CreateView(PageType::Url(
                "https://onthoudhetv3.weilers.nl/".to_string(),
            )))),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                self.tick_count += 1;
                if let State::Ticking { last_tick } = self.state {
                    let now = std::time::Instant::now();
                    self.tick_duration = now.duration_since(last_tick);
                    self.state = State::Ticking { last_tick: now };
                    if self.tick_count % 100 == 0 {
                        println!("Tick duration: {:?}", self.tick_duration);
                        // self.ready = false;
                        // let _ = self.webview.update(Action::ChangeView(0));
                    }
                }
                return Task::none();
            }
            
            Message::WebView(action) => {
                println!("WebView action: {:?}", action);
                self.webview.update(action)
            }
            Message::ViewCreated => {
                println!("WebView is ready!");
                self.ready = true;
                self.webview.update(Action::ChangeView(0))
            }
            Message::Screenshot => {
                if self.png_saving {
                    return Task::none();
                }
                return window::latest()
                    .and_then(window::screenshot)
                    .map(Message::Screenshotted);
            }
            Message::Screenshotted(screenshot) => {
                self.screenshot = Some((
                    screenshot.clone(),
                    image::Handle::from_rgba(
                        screenshot.size.width,
                        screenshot.size.height,
                        screenshot.rgba,
                    ),
                ));
                self.state = State::Ticking {
                    last_tick: std::time::Instant::now(),
                };
                return Task::done(Message::Png);
            }
            Message::Png => {
                if let Some((screenshot, _handle)) = &self.screenshot {
                    self.png_saving = true;

                    return Task::perform(save_to_png(screenshot.clone()), Message::PngSaved);
                }
                return Task::none();
            }
            Message::PngSaved(res) => {
                self.png_saving = false;
                self.saved_png_path = Some(res);
                return Task::done(Message::Scp);
            }
            Message::Scp => {
                if self.scp_sending {
                    return Task::none();
                }
                self.scp_sending = true;
                tokio::spawn(async {
                    if let Err(e) = pimoroni_notify().await {
                        eprintln!("Failed to send notification: {}", e);
                    }
                });
                return Task::none();
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        if self.ready {
            self.webview.view().map(Message::WebView)
        } else {
            iced::widget::text("Loading...").into()
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let tick = match self.state {
            State::Idle => Subscription::none(),
            State::Ticking { .. } => time::every(milliseconds(100)).map(|_| Message::Tick),
        };

        Subscription::batch(vec![
            tick,
            time::every(Duration::from_millis(10))
                .map(|_| Action::Update)
                .map(Message::WebView),
            if self.ready {
                time::every(Duration::from_secs(1)).map(|_| Message::Screenshot)
            } else {
                Subscription::none()
            },
            // time::every(Duration::from_secs(1)).map(|_| Message::Screenshot),
        ])
    }
}

fn main() -> iced::Result {
    // CEF requires this at the top of main()
    #[cfg(feature = "cef")]
    if iced_webview::cef_subprocess_check() {
        return Ok(());
    }

    // 800x480 window
    iced::application(App::new, App::update, App::view)
        .title("Webview")
        .window_size(iced::Size::new(800.0, 480.0))
        .subscription(App::subscription)
        .run()
}

async fn save_to_png(screenshot: Screenshot) -> Result<String, PngError> {
    let path = "screenshot.png".to_string();

    let _ = tokio::task::spawn_blocking(move || {
        img::save_buffer(
            &path,
            &screenshot.rgba,
            screenshot.size.width,
            screenshot.size.height,
            ColorType::Rgba8,
        )
        .map(|_| path)
        .map_err(|error| PngError(error.to_string()))
    })
    .await
    .expect("Blocking task to finish");

    // pimoroni_notify().await.expect("?");
    Ok("screenshot.png".to_string())
}

#[derive(Clone, Debug)]
struct PngError(String);

async fn pimoroni_notify() -> tokio::io::Result<()> {
    sh_exec("/home/david/onthoudhet-kiosk.sh").await
}

async fn sh_exec(command: &str) -> tokio::io::Result<()> {
    use tokio::process::Command;

    Command::new("bash")
        .arg("-c")
        .arg(command)
        .spawn()?
        .wait()
        .await
        .map(|_| ())
}
