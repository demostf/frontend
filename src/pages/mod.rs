pub mod about;
pub mod demo;
pub mod index;
mod plugin_section;
pub mod upload;

use crate::asset::saved_asset_url;
use crate::session::SessionData;
use maud::{html, Markup, DOCTYPE};
use std::borrow::Cow;

pub trait Page {
    fn title(&self) -> Cow<'static, str>;
    fn render(&self) -> Markup;
}

pub fn render<T: Page>(page: T, session: SessionData) -> Markup {
    let style_url = saved_asset_url!("style.css");
    html! {
        (DOCTYPE)
        html {
            head {
                title { (page.title()) }
                link rel="stylesheet" type="text/css" href=(style_url);
                link rel="shortcut icon" type="image/svg+xml" href="images/logo.svg";
            }
            body {
                header {
                    span .main {
                        a href = "/" { "demos.tf" }
                    }
                    span { a href = "/about" { "about" } }
                    span { a href = "/viewer" { "viewer" } }
                    span.beta { a href = "/editor" { "editor" } }
                    @if let SessionData::Authenticated(user) = session {
                        span.right { a href = "/logout" { "Logout" } }
                        span.right { a href = "/upload" { "Upload" } }
                        span.right { a href = (user.steam_id.profile_link()) { (user.name) } }
                    } @else {
                        span.right { a.steam-login href = "/login" { "Sign in through Steam" } }
                    }
                }
                .page { (page.render()) }
            }
            footer {
                "©"
                a href = "https://steamcommunity.com/id/icewind1991" { "Icewind" }
                " 2017."
            }
        }
    }
}
