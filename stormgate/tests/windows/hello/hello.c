/* Storm Gate synthetic test: console output and exit code.
 * SPDX-License-Identifier: Apache-2.0 OR MIT */
#include <stdio.h>
#include <windows.h>

int main(void)
{
    OSVERSIONINFOW ver = { sizeof(ver) };
    SYSTEM_INFO si;
    char path[MAX_PATH];

    GetNativeSystemInfo(&si);
    if (!GetModuleFileNameA(NULL, path, sizeof(path)))
    {
        printf("GetModuleFileNameA failed: %lu\n", GetLastError());
        return 2;
    }
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wdeprecated-declarations"
    GetVersionExW(&ver);
#pragma GCC diagnostic pop

    printf("hello from %s\n", path);
    printf("windows %lu.%lu build %lu, arch %u, %lu cpus\n", ver.dwMajorVersion,
           ver.dwMinorVersion, ver.dwBuildNumber, si.wProcessorArchitecture,
           si.dwNumberOfProcessors);
    printf("STORMGATE-TEST-PASS hello\n");
    fflush(stdout);
    return 0;
}
