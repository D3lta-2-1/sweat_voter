use crate::common::{Credentials, ProfilID};
use crate::{list_classes, login};
use dioxus::fullstack::Form;
use dioxus::prelude::*;
use dioxus::router::{FromHashFragment, RouterConfig};
use dioxus_html::img;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;

#[rustfmt::skip]
#[derive(Clone, Routable)]
pub enum Route {
    #[route("/")]
    Home{},
    #[route("/login")]
    LoginPage{},
    #[nest("/class/:class_name")]
        #[route("")]
        ClassPageDefault{
            class_name: String,
        },
        #[route("/:id")]
        ClassPageWithId {
            class_name: String,
            id: u32
        }

}

#[component]
pub fn Home() -> Element {
    let classes = use_resource(list_classes);

    rsx! {
        if let Some(classes) = classes() {
            for class in classes?.iter() {
                Link {
                    to: Route::ClassPageDefault {
                        class_name: class.clone(),
                    },
                    "{class}"
                }
            }
        }

        //LoginBar {}
        div { "Home" }
    }
}

#[component]
pub fn App() -> Element {
    rsx! {
        Router::<Route> { config: || RouterConfig::default() }
    }
}

#[component]
pub fn ClassPageDefault(class_name: String) -> Element {
    rsx! {
        ClassPage { class_name, profil_id: None }
    }
}

#[component]
pub fn ClassPageWithId(class_name: String, id: u32) -> Element {
    rsx! {
        ClassPage { class_name, profil_id: Some(id) }
    }
}

#[component]
pub fn ClassPage(class_name: String, profil_id: Option<u32>) -> Element {
    rsx! { "class name: {class_name}" }
}

static CSS: Asset = asset!("/assets/main.css");

#[component]
pub fn LoginPage() -> Element {
    rsx!(
        document::Stylesheet { href: CSS }
        form {
            onsubmit: move |evt: FormEvent| async move {
                evt.prevent_default();

                let values: Credentials = evt.parsed_values().expect("failed to parse values");

                info!("trying to log");

                if login(Form(values)).await? {
                    navigator().replace(Route::Home {});
                }
                Ok(())
            },
            div { class: "navbar",
                img {
                    class: "navbar_logo",
                    src: asset!("/assets/images/logo-corneille.svg"),
                }
                h3 { class: "titre", "Prépa Pierre Corneille" }
                li { class: "navbar_element", "Home" }
                li { class: "navbar_element", "Sweat" }
            
            }

            div { class: "auth",
                h1 { "Identification" }
            }

            div { class: "login_container",
                div { class: "login_box",
                    div {
                        label { "Identifiants" }
                    }
                    div {
                        input { r#type: "text", id: "name", name: "name" }
                    }

                    div {
                        label { "Mot de passe" }
                    }
                    div {
                        input {
                            r#type: "password",
                            id: "password",
                            name: "password",
                        }
                    }

                    div { class: "login_button_container",
                        button { class: "login_button", "Confirmer" }
                    }
                }
            }
        }
    )
}
