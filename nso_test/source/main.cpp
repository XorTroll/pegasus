#include <switch.h>
#include <cstdio>

extern "C" {

    void Do1() {
        const auto msg = "Hello pegasus from libnx!";
        svcOutputDebugString(msg, __builtin_strlen(msg));

        while(true) {
            svcSleepThread(10'000'000);
        }
    }

    void Do2() {
        char msg[0x200] = {};
        const auto len = sprintf(msg, "Hello %d!", 12);

        svcOutputDebugString(msg, len);

        while(true) {
            svcSleepThread(10'000'000);
        }
    }

    #define LOG_SVC_OUT(msg) svcOutputDebugString(msg, __builtin_strlen(msg))

    void __libnx_init(void* ctx, Handle main_thread, void* saved_lr) {
        hosversionSet(MAKEHOSVERSION(5, 1, 0));
        
        auto rc = smInitialize();
        if(R_FAILED(rc)) {
            diagAbortWithResult(rc);
        }

        LOG_SVC_OUT("Initialized sm!");

        rc = setsysInitialize();
        if(R_FAILED(rc)) {
            diagAbortWithResult(rc);
        }

        LOG_SVC_OUT("Initialized setsys!");

        SetSysFirmwareVersion fwv;
        rc = setsysGetFirmwareVersion(&fwv);
        if(R_FAILED(rc)) {
            diagAbortWithResult(rc);
        }

        LOG_SVC_OUT("Got fw version!");

        LOG_SVC_OUT("Hello pegasus from libnx!");
    }

}

int main() {}