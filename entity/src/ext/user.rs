use crate::entities::user;
use lettre::address::AddressError;
use lettre::message::Mailbox;

impl user::Model {
    pub fn get_mailbox(&self) -> Result<Mailbox, AddressError> {
        Ok(Mailbox::new(Some(self.name.clone()), self.email.parse()?))
    }
}
