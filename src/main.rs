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
}

struct App {
    webview: WebView<Engine, Message>,
    ready: bool,
    screenshot: Option<(Screenshot, image::Handle)>,
    saved_png_path: Option<Result<String, PngError>>,
    png_saving: bool,
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
            },
            Task::done(Message::WebView(Action::CreateView(PageType::Url(
                "https://onthoudhetv2.weilers.nl/".to_string(),
            )))),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WebView(action) => self.webview.update(action),
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
        Subscription::batch(vec![
            time::every(Duration::from_millis(10))
                .map(|_| Action::Update)
                .map(Message::WebView),
            time::every(Duration::from_secs(1)).map(|_| Message::Screenshot),
        ])
    }
}

fn main() -> iced::Result {
    // CEF requires this at the top of main()
    #[cfg(feature = "cef")]
    if iced_webview::cef_subprocess_check() {
        return Ok(());
    }

    iced::application(App::new, App::update, App::view)
        .title("Webview")
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
