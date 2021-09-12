
#pragma once
#include <pg/pg_Result.hpp>

namespace pg::kern {

    namespace result {

        PG_RESULT_NAMESPACE_DEFINE_MODULE(1);

        PG_RESULT_NAMESPACE_DEFINE(OutOfSessions, 7);

        PG_RESULT_NAMESPACE_DEFINE(InvalidArgument, 14);

        PG_RESULT_NAMESPACE_DEFINE(NotImplemented, 33);

        PG_RESULT_NAMESPACE_DEFINE(StopProcessingException, 54);

        PG_RESULT_NAMESPACE_DEFINE(NoSynchronizationObject, 57);

        PG_RESULT_NAMESPACE_DEFINE(TerminationRequested, 59);

        PG_RESULT_NAMESPACE_DEFINE(NoEvent, 70);

        PG_RESULT_NAMESPACE_DEFINE(InvalidSize, 101);
        PG_RESULT_NAMESPACE_DEFINE(InvalidAddress, 102);
        PG_RESULT_NAMESPACE_DEFINE(OutOfResource, 103);
        PG_RESULT_NAMESPACE_DEFINE(OutOfMemory, 104);
        PG_RESULT_NAMESPACE_DEFINE(OutOfHandles, 105);
        PG_RESULT_NAMESPACE_DEFINE(InvalidCurrentMemory, 106);

        PG_RESULT_NAMESPACE_DEFINE(InvalidNewMemoryPermission, 108);

        PG_RESULT_NAMESPACE_DEFINE(InvalidMemoryRegion, 110);

        PG_RESULT_NAMESPACE_DEFINE(InvalidPriority, 112);
        PG_RESULT_NAMESPACE_DEFINE(InvalidCoreId, 113);
        PG_RESULT_NAMESPACE_DEFINE(InvalidHandle, 114);
        PG_RESULT_NAMESPACE_DEFINE(InvalidPointer, 115);
        PG_RESULT_NAMESPACE_DEFINE(InvalidCombination, 116);
        PG_RESULT_NAMESPACE_DEFINE(TimedOut, 117);
        PG_RESULT_NAMESPACE_DEFINE(Cancelled, 118);
        PG_RESULT_NAMESPACE_DEFINE(OutOfRange, 119);
        PG_RESULT_NAMESPACE_DEFINE(InvalidEnumValue, 120);
        PG_RESULT_NAMESPACE_DEFINE(NotFound, 121);
        PG_RESULT_NAMESPACE_DEFINE(Busy, 122);
        PG_RESULT_NAMESPACE_DEFINE(SessionClosed, 123);
        PG_RESULT_NAMESPACE_DEFINE(NotHandled, 124);
        PG_RESULT_NAMESPACE_DEFINE(InvalidState, 125);
        PG_RESULT_NAMESPACE_DEFINE(ReservedUsed, 126);
        PG_RESULT_NAMESPACE_DEFINE(NotSupported, 127);
        PG_RESULT_NAMESPACE_DEFINE(Debug, 128);
        PG_RESULT_NAMESPACE_DEFINE(NoThread, 129);
        PG_RESULT_NAMESPACE_DEFINE(UnknownThread, 130);
        PG_RESULT_NAMESPACE_DEFINE(PortClosed, 131);
        PG_RESULT_NAMESPACE_DEFINE(LimitReached, 132);
        PG_RESULT_NAMESPACE_DEFINE(InvalidMemoryPool, 133);

        PG_RESULT_NAMESPACE_DEFINE(ReceiveListBroken, 258);
        PG_RESULT_NAMESPACE_DEFINE(OutOfAddressSpace, 259);
        PG_RESULT_NAMESPACE_DEFINE(MessageTooLarge, 260);

        PG_RESULT_NAMESPACE_DEFINE(InvalidProcessId, 517);
        PG_RESULT_NAMESPACE_DEFINE(InvalidThreadId, 518);
        PG_RESULT_NAMESPACE_DEFINE(InvalidId, 519);
        PG_RESULT_NAMESPACE_DEFINE(ProcessTerminated, 520);

    }

}