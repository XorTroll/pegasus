use super::*;
use crate::util::Shared;
use crate::ipc::sf;
use super::result;

#[macro_export]
macro_rules! ipc_client_send_request_command {
    ([$session:expr; $rq_id:expr] ( $( $in_param:expr ),* ) => ( $( $out_param:ident: $out_param_type:ty ),* )) => {{
        let rc: $crate::result::Result<_> = {
            let mut ctx = $crate::ipc::CommandContext::new_client($session);

            let mut walker = $crate::ipc::DataWalker::new(core::ptr::null_mut());
            $(
                {
                    let in_v = &$in_param;
                    $crate::ipc::client::CommandParameter::<_>::before_request_write(in_v, &mut walker, &mut ctx)?;
                }
            )*
            ctx.in_params.data_size = walker.get_offset() as u32;
            
            match $session.protocol {
                $crate::ipc::CommandProtocol::Cmif => $crate::ipc::cmif::client::write_request_command_on_ipc_buffer(&mut ctx, Some($rq_id), $crate::ipc::cmif::DomainCommandType::SendMessage),
                $crate::ipc::CommandProtocol::Tipc => $crate::ipc::tipc::client::write_request_command_on_ipc_buffer(&mut ctx, $rq_id)
            };

            walker.reset_with(ctx.in_params.data_offset);
            $(
                {
                    let in_v = &$in_param;
                    $crate::ipc::client::CommandParameter::<_>::before_send_sync_request(in_v, &mut walker, &mut ctx)?;
                }
            )*

            $crate::kern::svc::send_sync_request($session.handle)?;

            match $session.protocol {
                $crate::ipc::CommandProtocol::Cmif => $crate::ipc::cmif::client::read_request_command_response_from_ipc_buffer(&mut ctx)?,
                $crate::ipc::CommandProtocol::Tipc => $crate::ipc::tipc::client::read_request_command_response_from_ipc_buffer(&mut ctx)?
            };

            walker.reset_with(ctx.out_params.data_offset);
            $( let $out_param = <$out_param_type as $crate::ipc::client::CommandParameter<_>>::after_response_read(&mut walker, &mut ctx)?; )*

            Ok(( $( $out_param ),* ))
        };
        rc
    }};
}

#[macro_export]
macro_rules! ipc_client_send_control_command {
    ([$session:expr; $control_rq_id:expr] ( $( $in_param:expr ),* ) => ( $( $out_param:ident: $out_param_type:ty ),* )) => {{
        let rc: $crate::result::Result<_> = {
            if $session.uses_tipc_protocol() {
                /* Err */
            }

            let mut ctx = $crate::ipc::CommandContext::new_client($session);

            let mut walker = $crate::ipc::DataWalker::new(core::ptr::null_mut());
            $(
                {
                    let in_v = &$in_param;
                    $crate::ipc::client::CommandParameter::<_>::before_request_write(in_v, &mut walker, &mut ctx)?;
                }
            )*
            ctx.in_params.data_size = walker.get_offset() as u32;
            
            $crate::ipc::cmif::client::write_control_command_on_ipc_buffer(&mut ctx, $control_rq_id);

            walker.reset_with(ctx.in_params.data_offset);
            $(
                {
                    let in_v = &$in_param;
                    $crate::ipc::client::CommandParameter::<_>::before_send_sync_request(in_v, &mut walker, &mut ctx)?;
                }
            )*

            $crate::kern::svc::send_sync_request($session.handle)?;

            $crate::ipc::cmif::client::read_control_command_response_from_ipc_buffer(&mut ctx)?;

            walker.reset_with(ctx.out_params.data_offset);
            $( let $out_param = <$out_param_type as $crate::ipc::client::CommandParameter<_>>::after_response_read(&mut walker, &mut ctx)?; )*

            Ok(( $( $out_param ),* ))
        };
        rc
    }};
}

pub trait CommandParameter<O> {
    fn before_request_write(var: &Self, walker: &mut DataWalker, ctx: &mut CommandContext) -> Result<()>;
    fn before_send_sync_request(var: &Self, walker: &mut DataWalker, ctx: &mut CommandContext) -> Result<()>;
    fn after_response_read(walker: &mut DataWalker, ctx: &mut CommandContext) -> Result<O>;
}

impl<T: Copy> CommandParameter<T> for T {
    default fn before_request_write(_raw: &Self, walker: &mut DataWalker, _ctx: &mut CommandContext) -> Result<()> {
        walker.advance::<Self>();
        Ok(())
    }

    default fn before_send_sync_request(raw: &Self, walker: &mut DataWalker, _ctx: &mut CommandContext) -> Result<()> {
        walker.advance_set(*raw);
        Ok(())
    }

    default fn after_response_read(walker: &mut DataWalker, _ctx: &mut CommandContext) -> Result<Self> {
        Ok(walker.advance_get())
    }
}

impl<const A: BufferAttribute, const S: usize> CommandParameter<sf::Buffer<A, S>> for sf::Buffer<A, S> {
    fn before_request_write(buffer: &Self, _walker: &mut DataWalker, ctx: &mut CommandContext) -> Result<()> {
        ctx.add_buffer(buffer.clone())
    }

    fn before_send_sync_request(_buffer: &Self, _walker: &mut DataWalker, _ctx: &mut CommandContext) -> Result<()> {
        Ok(())
    }

    fn after_response_read(_walker: &mut DataWalker, _ctx: &mut CommandContext) -> Result<Self> {
        // Buffers aren't returned as output variables - the buffer sent as input (with Out attribute) will contain the output data
        result::ResultUnsupportedOperation::make_err()
    }
}

impl<const M: HandleMode> CommandParameter<sf::Handle<M>> for sf::Handle<M> {
    fn before_request_write(handle: &Self, _walker: &mut DataWalker, ctx: &mut CommandContext) -> Result<()> {
        ctx.in_params.add_handle(handle.clone())
    }

    fn before_send_sync_request(_handle: &Self, _walker: &mut DataWalker, _ctx: &mut CommandContext) -> Result<()> {
        Ok(())
    }

    fn after_response_read(_walker: &mut DataWalker, ctx: &mut CommandContext) -> Result<Self> {
        ctx.out_params.pop_handle()
    }
}

impl CommandParameter<sf::ProcessId> for sf::ProcessId {
    fn before_request_write(_process_id: &Self, walker: &mut DataWalker, ctx: &mut CommandContext) -> Result<()> {
        ctx.in_params.send_process_id = true;
        if ctx.object_info.uses_cmif_protocol() {
            // TIPC doesn't set this placeholder space for process IDs
            walker.advance::<u64>();
        }
        Ok(())
    }

    fn before_send_sync_request(process_id: &Self, walker: &mut DataWalker, ctx: &mut CommandContext) -> Result<()> {
        // Same as above
        if ctx.object_info.uses_cmif_protocol() {
            walker.advance_set(process_id.process_id);
        }
        Ok(())
    }

    fn after_response_read(_walker: &mut DataWalker, _ctx: &mut CommandContext) -> Result<Self> {
        // TODO: is this actually valid/used?
        result::ResultUnsupportedOperation::make_err()
    }
}

impl CommandParameter<Shared<dyn sf::IObject>> for Shared<dyn sf::IObject> {
    fn before_request_write(session: &Self, _walker: &mut DataWalker, ctx: &mut CommandContext) -> Result<()> {
        ctx.in_params.add_object(session.get().get_info())
    }

    fn before_send_sync_request(_session: &Self, _walker: &mut DataWalker, _ctx: &mut CommandContext) -> Result<()> {
        Ok(())
    }

    fn after_response_read(_walker: &mut DataWalker, _ctx: &mut CommandContext) -> Result<Self> {
        // Only supported when the IObject type is known (see the generic implementation below)
        result::ResultUnsupportedOperation::make_err()
    }
}

impl<S: sf::client::IClientObject + 'static> CommandParameter<Shared<dyn sf::IObject>> for Shared<S> {
    fn before_request_write(session: &Self, _walker: &mut DataWalker, ctx: &mut CommandContext) -> Result<()> {
        ctx.in_params.add_object(session.get().get_info())
    }

    fn before_send_sync_request(_session: &Self, _walker: &mut DataWalker, _ctx: &mut CommandContext) -> Result<()> {
        Ok(())
    }

    fn after_response_read(_walker: &mut DataWalker, ctx: &mut CommandContext) -> Result<Shared<dyn sf::IObject>> {
        let object_info = ctx.pop_object()?;
        Ok(Shared::new(S::new(sf::Session::from(object_info))))
    }
}