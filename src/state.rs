use axum::extract::FromRef;

use std::sync::{Arc, RwLock};

use crate::actions;
use crate::db;
use crate::fr;
use crate::pages;
use crate::Config;

pub struct PlainchantState<DB: db::Database, FR: fr::FileRack> {
    pub config:  Arc<Config>,
    pub sp:      Arc<pages::StaticPages>,
    pub pages:   Arc<RwLock<pages::Pages>>,
    pub actions: Arc<actions::Actions>,
    pub db:      Arc<DB>,
    pub fr:      Arc<FR>,
}

impl<DB: db::Database, FR: fr::FileRack> FromRef<PlainchantState<DB, FR>> for Arc<Config> {
    fn from_ref(state: &PlainchantState<DB, FR>) -> Self {
        state.config.clone()
    }
}

impl<DB: db::Database, FR: fr::FileRack> FromRef<PlainchantState<DB, FR>>
    for Arc<pages::StaticPages>
{
    fn from_ref(state: &PlainchantState<DB, FR>) -> Self {
        state.sp.clone()
    }
}

impl<DB: db::Database, FR: fr::FileRack> FromRef<PlainchantState<DB, FR>>
    for Arc<RwLock<pages::Pages>>
{
    fn from_ref(state: &PlainchantState<DB, FR>) -> Self {
        state.pages.clone()
    }
}

impl<DB: db::Database, FR: fr::FileRack> FromRef<PlainchantState<DB, FR>>
    for Arc<actions::Actions>
{
    fn from_ref(state: &PlainchantState<DB, FR>) -> Self {
        state.actions.clone()
    }
}

pub struct DbState<DB: db::Database> {
    pub db: Arc<DB>,
}

impl<DB: db::Database, FR: fr::FileRack> FromRef<PlainchantState<DB, FR>> for DbState<DB> {
    fn from_ref(state: &PlainchantState<DB, FR>) -> Self {
        Self {
            db: state.db.clone(),
        }
    }
}

pub struct FrState<FR: fr::FileRack> {
    pub fr: Arc<FR>,
}

impl<DB: db::Database, FR: fr::FileRack> FromRef<PlainchantState<DB, FR>> for FrState<FR> {
    fn from_ref(state: &PlainchantState<DB, FR>) -> Self {
        Self {
            fr: state.fr.clone(),
        }
    }
}

impl<DB: db::Database, FR: fr::FileRack> Clone for PlainchantState<DB, FR> {
    fn clone(&self) -> Self {
        PlainchantState {
            config:  self.config.clone(),
            sp:      self.sp.clone(),
            pages:   self.pages.clone(),
            actions: self.actions.clone(),
            db:      self.db.clone(),
            fr:      self.fr.clone(),
        }
    }
}

impl<DB: db::Database, FR: fr::FileRack> PlainchantState<DB, FR> {
    pub fn new(
        config: Config,
        sp: pages::StaticPages,
        pages: pages::Pages,
        actions: actions::Actions,
        db: DB,
        fr: FR,
    ) -> Self {
        PlainchantState {
            config:  Arc::new(config),
            sp:      Arc::new(sp),
            pages:   Arc::new(RwLock::new(pages)),
            actions: Arc::new(actions),
            db:      Arc::new(db),
            fr:      Arc::new(fr),
        }
    }
}
