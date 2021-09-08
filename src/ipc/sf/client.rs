use crate::ipc::sf;
use crate::kern::svc;
use crate::result::*;
use crate::util::Shared;

pub mod sm;
use crate::sm::ServiceName;
use super::sm::IUserInterface;

pub trait IClientObject: sf::IObject {
    fn new(session: sf::Session) -> Self where Self: Sized;
}

pub trait INamedPort: IClientObject {
    fn get_name() -> &'static str;
    fn post_initialize(&mut self) -> Result<()>;
}

pub trait IService: IClientObject {
    fn get_name() -> &'static str;
    fn as_domain() -> bool;
    fn post_initialize(&mut self) -> Result<()>;
}

pub fn new_named_port_object<T: INamedPort + 'static>() -> Result<Shared<T>> {
    let handle = svc::connect_to_named_port(T::get_name())?;
    let mut object = T::new(sf::Session::from_handle(handle));
    object.post_initialize()?;
    Ok(Shared::new(object))
}

pub fn new_service_object<T: IService + 'static>() -> Result<Shared<T>> {
    let sm = new_named_port_object::<sm::UserInterface>()?;
    let session_handle = sm.get().get_service_handle(ServiceName::new(T::get_name()))?;
    let mut object = T::new(sf::Session::from_handle(session_handle.handle));
    if T::as_domain() {
        object.convert_to_domain()?;
    }
    object.post_initialize()?;
    Ok(Shared::new(object))
}