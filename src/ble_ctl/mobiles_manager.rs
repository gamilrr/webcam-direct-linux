use std::collections::HashMap;

use crate::app_data::{AppDataStore, MobileId, MobileSchema};

use crate::error::Result;

type Address = String;

pub struct MobilesManager<Db>
where
    Db: AppDataStore,
{
    app_db: Db,
    connected_mobiles: HashMap<Address, MobileId>,
    msg_buffer: HashMap<Address, String>,
}

impl<Db> MobilesManager<Db>
where
    Db: AppDataStore,
{
    pub fn new(app_db: Db) -> Self {
        Self {
            app_db,
            connected_mobiles: HashMap::new(),
            msg_buffer: HashMap::new(),
        }
    }
}
