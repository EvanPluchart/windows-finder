//! Desinstallation depuis "Applications" : confirmation, recherche de la commande officielle, lancement.

use gpui::{
    AnyElement, Context, InteractiveElement, IntoElement, MouseButton, ParentElement, StatefulInteractiveElement, Styled,
    div, prelude::*, px, };
use crate::theme::{rgb, rgba};

use crate::{apps, theme, Finder};

pub struct Confirm {
    pub name: String,
}

impl Finder {
    /// Vrai (avec une pastille d'explication) si l'emplacement courant est virtuel : pas de modification de fichiers.
    pub fn read_only_place(&mut self, cx: &mut Context<Self>) -> bool {
        if crate::fs::is_virtual(&self.cwd) {
            self.show_notice("Cet emplacement est en lecture seule.", cx);
            return true;
        }
        false
    }

    pub fn ask_uninstall(&mut self, cx: &mut Context<Self>) {
        if self.cwd.to_str() != Some(crate::fs::APPS) {
            return;
        }
        if let Some(e) = self.selected_entry() {
            self.confirm = Some(Confirm { name: e.name.to_string() });
            cx.notify();
        }
    }

    pub fn run_uninstall(&mut self, cx: &mut Context<Self>) {
        let Some(c) = self.confirm.take() else { return };
        self.show_notice(format!("Recherche du programme de d\u{e9}sinstallation de \u{ab} {} \u{bb}\u{2026}", c.name), cx);
        let name = c.name.clone();
        cx.spawn(async move |this, cx| {
            let lookup = name.clone();
            let cmd = cx.background_executor().spawn(async move { apps::find_uninstall(&lookup) }).await;
            this.update(cx, |f, cx| match cmd {
                Some(cmd) => {
                    apps::launch(&cmd);
                    f.show_notice(format!("D\u{e9}sinstallation de \u{ab} {name} \u{bb} lanc\u{e9}e"), cx);
                }
                None => f.show_notice(
                    format!(
                        "Aucun programme de d\u{e9}sinstallation pour \u{ab} {name} \u{bb} \u{2014} voir Param\u{e8}tres > Applications"
                    ),
                    cx,
                ),
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub fn render_confirm(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let c = self.confirm.as_ref()?;
        let button = |id: &'static str, text: &'static str, primary: bool| {
            div()
                .id(id)
                .px(px(14.))
                .h(px(28.))
                .flex()
                .items_center()
                .rounded(px(7.))
                .text_size(px(13.))
                .when(primary, |d| d.bg(rgb(0xd7342b)).text_color(rgb(0xfffffe)))
                .when(!primary, |d| d.bg(rgb(0xe9e9ec)).text_color(rgb(theme::TEXT)))
                .child(text)
        };
        Some(
            div()
                .id("confirm-backdrop")
                .absolute()
                .inset_0()
                .flex()
                .justify_center()
                .items_start()
                .pt(px(120.))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _, cx| {
                        this.confirm = None;
                        cx.notify();
                    }),
                )
                .child(
                    div()
                        .w(px(360.))
                        .p(px(20.))
                        .flex()
                        .flex_col()
                        .gap(px(12.))
                        .rounded(px(12.))
                        .bg(rgba(0xf6f6f6fc))
                        .border_1()
                        .border_color(rgb(0xd9d9de))
                        .shadow_lg()
                        .occlude()
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(rgb(theme::TEXT))
                                .child(format!("D\u{e9}sinstaller \u{ab} {} \u{bb} ?", c.name)),
                        )
                        .child(
                            div().text_size(px(13.)).text_color(rgb(theme::TEXT_SECONDARY)).child(
                                "Le programme de d\u{e9}sinstallation officiel va s'ouvrir et pourra demander une confirmation.",
                            ),
                        )
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap(px(8.))
                                .child(button("confirm-cancel", "Annuler", false).on_click(cx.listener(|this, _, _, cx| {
                                    this.confirm = None;
                                    cx.notify();
                                })))
                                .child(
                                    button("confirm-ok", "D\u{e9}sinstaller", true)
                                        .on_click(cx.listener(|this, _, _, cx| this.run_uninstall(cx))),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }
}
