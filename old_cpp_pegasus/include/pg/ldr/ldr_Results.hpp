
#pragma once
#include <pg/pg_Result.hpp>

namespace pg::ldr {

    namespace result {

        PG_RESULT_NAMESPACE_DEFINE_MODULE(9);

        PG_RESULT_NAMESPACE_DEFINE(TooLongArgument, 1);
        PG_RESULT_NAMESPACE_DEFINE(TooManyArguments, 2);
        PG_RESULT_NAMESPACE_DEFINE(TooLargeMeta, 3);
        PG_RESULT_NAMESPACE_DEFINE(InvalidMeta, 4);
        PG_RESULT_NAMESPACE_DEFINE(InvalidNso, 5);
        PG_RESULT_NAMESPACE_DEFINE(InvalidPath, 6);
        PG_RESULT_NAMESPACE_DEFINE(TooManyProcesses, 7);
        PG_RESULT_NAMESPACE_DEFINE(NotPinned, 8);
        PG_RESULT_NAMESPACE_DEFINE(InvalidProgramId, 9);
        PG_RESULT_NAMESPACE_DEFINE(InvalidVersion, 10);
        PG_RESULT_NAMESPACE_DEFINE(InvalidAcidSignature, 11);
        PG_RESULT_NAMESPACE_DEFINE(InvalidNcaSignature, 12);

        PG_RESULT_NAMESPACE_DEFINE(InsufficientAddressSpace, 51);
        PG_RESULT_NAMESPACE_DEFINE(InvalidNro, 52);
        PG_RESULT_NAMESPACE_DEFINE(InvalidNrr, 53);
        PG_RESULT_NAMESPACE_DEFINE(InvalidSignature, 54);
        PG_RESULT_NAMESPACE_DEFINE(InsufficientNroRegistrations, 55);
        PG_RESULT_NAMESPACE_DEFINE(InsufficientNrrRegistrations, 56);
        PG_RESULT_NAMESPACE_DEFINE(NroAlreadyLoaded, 57);

        PG_RESULT_NAMESPACE_DEFINE(InvalidAddress, 81);
        PG_RESULT_NAMESPACE_DEFINE(InvalidSize, 82);
        PG_RESULT_NAMESPACE_DEFINE(NotLoaded, 84);
        PG_RESULT_NAMESPACE_DEFINE(NotRegistered, 85);
        PG_RESULT_NAMESPACE_DEFINE(InvalidSession, 86);
        PG_RESULT_NAMESPACE_DEFINE(InvalidProcess, 87);

        PG_RESULT_NAMESPACE_DEFINE(UnknownCapability, 100);
        PG_RESULT_NAMESPACE_DEFINE(InvalidCapabilityKernelFlags, 103);
        PG_RESULT_NAMESPACE_DEFINE(InvalidCapabilitySyscallMask, 104);
        PG_RESULT_NAMESPACE_DEFINE(InvalidCapabilityMapRange, 106);
        PG_RESULT_NAMESPACE_DEFINE(InvalidCapabilityMapPage, 107);
        PG_RESULT_NAMESPACE_DEFINE(InvalidCapabilityMapRegion, 110);
        PG_RESULT_NAMESPACE_DEFINE(InvalidCapabilityInterruptPair, 111);
        PG_RESULT_NAMESPACE_DEFINE(InvalidCapabilityApplicationType, 113);
        PG_RESULT_NAMESPACE_DEFINE(InvalidCapabilityKernelVersion, 114);
        PG_RESULT_NAMESPACE_DEFINE(InvalidCapabilityHandleTable, 115);
        PG_RESULT_NAMESPACE_DEFINE(InvalidCapabilityDebugFlags, 116);

        PG_RESULT_NAMESPACE_DEFINE(InternalError, 200);

    }

}