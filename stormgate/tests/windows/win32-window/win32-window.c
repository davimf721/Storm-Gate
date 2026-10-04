/* Storm Gate synthetic test: create a window, pump messages, close it.
 * Exercises winemac.drv window creation without any graphics API.
 * SPDX-License-Identifier: Apache-2.0 OR MIT */
#include <stdio.h>
#include <windows.h>

static int painted;

static LRESULT CALLBACK wndproc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp)
{
    switch (msg)
    {
    case WM_PAINT:
    {
        PAINTSTRUCT ps;
        HDC dc = BeginPaint(hwnd, &ps);
        FillRect(dc, &ps.rcPaint, (HBRUSH)GetStockObject(BLACK_BRUSH));
        EndPaint(hwnd, &ps);
        painted = 1;
        return 0;
    }
    case WM_TIMER:
        DestroyWindow(hwnd);
        return 0;
    case WM_DESTROY:
        PostQuitMessage(0);
        return 0;
    }
    return DefWindowProcW(hwnd, msg, wp, lp);
}

int main(void)
{
    WNDCLASSW wc = { 0 };
    HWND hwnd;
    MSG msg;

    wc.lpfnWndProc = wndproc;
    wc.hInstance = GetModuleHandleW(NULL);
    wc.lpszClassName = L"StormGateTest";
    wc.hCursor = LoadCursorW(NULL, (LPCWSTR)IDC_ARROW);
    if (!RegisterClassW(&wc))
    {
        printf("RegisterClassW failed: %lu\n", GetLastError());
        return 2;
    }
    hwnd = CreateWindowW(L"StormGateTest", L"Storm Gate win32 test", WS_OVERLAPPEDWINDOW,
                         CW_USEDEFAULT, CW_USEDEFAULT, 640, 360, NULL, NULL, wc.hInstance, NULL);
    if (!hwnd)
    {
        printf("CreateWindowW failed: %lu\n", GetLastError());
        return 3;
    }
    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);
    SetTimer(hwnd, 1, 1500, NULL);

    while (GetMessageW(&msg, NULL, 0, 0) > 0)
    {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    if (!painted)
    {
        printf("window was never painted\n");
        return 4;
    }
    printf("STORMGATE-TEST-PASS win32-window\n");
    fflush(stdout);
    return 0;
}
