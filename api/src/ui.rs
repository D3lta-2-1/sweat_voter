use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use dioxus::fullstack::Form;
use dioxus::prelude::*;
use dioxus::router::{FromHashFragment, RouterConfig};
use crate::common::{Credentials, ProfilID};
use crate::{list_classes, login};



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
        ClassPage {
            class_name: class_name,
            profil_id: None
        }
    }
}

#[component]
pub fn ClassPageWithId(class_name: String, id: u32) -> Element {
    rsx! {
        ClassPage {
            class_name: class_name,
            profil_id: Some(id)
        }
    }
}

#[component]
pub fn ClassPage(class_name: String, profil_id: Option<u32>) -> Element {
    rsx! {
        "class name: {class_name}",
    }
}

#[component]
pub fn LoginPage() -> Element {
    rsx!(
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
            input { r#type: "text", id: "name", name: "name" }
            label { "NOM Prenom" }
            input { r#type: "password", id: "password", name: "password" }
            label { "Mot de passe" }
            button { "Login" }
        }
    )
}