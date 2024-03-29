use crate::data::user::Token;
use crate::pages::plugin_section::PluginSection;
use crate::pages::Page;
use demostf_build::Asset;
use maud::{html, Markup};
use std::borrow::Cow;

#[derive(Debug)]
pub struct UploadPage<'a> {
    pub key: &'a Token,
    pub api: &'a str,
}

impl<'a> UploadPage<'a> {
    pub fn plugin_section(&self) -> PluginSection<'a> {
        PluginSection {
            key: Some(self.key),
        }
    }
}

#[derive(Asset)]
#[asset(source = "script/upload.ts", url = "/upload.js")]
pub struct UploadScript;

impl Page for UploadPage<'_> {
    fn title(&self) -> Cow<'static, str> {
        "Upload - demos.tf".into()
    }

    fn render(&self) -> Markup {
        let script = UploadScript::url();
        html! {
            .upload-page {
                section.upload {
                    .teams {
                        .red {
                            input type = "text" name = "red" placeholder = "RED";
                        }
                        .blue {
                            input type = "text" name = "blue" placeholder = "BLU";
                        }
                        .clearfix {}
                    }
                    .dropzone role = "button" {
                        noscript {
                            "Javascript is required for demo upload."
                        }
                        span.text { "Drop files or click to upload" }
                        input type = "file" {}
                    }
                    .demo-info {
                        span.map {}
                        span.time {}
                    }
                    input type = "hidden" name = "api" value = (self.api) {}
                    button.button.button-primary disabled { "Upload" }
                }
                section {
                    .title {
                        h3 { "API Key" }
                    }
                    pre.key { (self.key) }
                    p { "This key is used by the plugin to authenticate you as the uploader and link the uploaded demo to your account." }
                }
                (self.plugin_section())
            }
            script defer src = (script) type = "text/javascript" {}
        }
    }
}
