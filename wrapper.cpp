#define WOW64EXT_NO_DLLMAIN
#include "wow64ext.cpp"

extern HANDLE g_heap;
extern BOOL g_isWow64;

extern "C" __declspec(dllexport) void __cdecl Wow64ExtInitialize()
{
    IsWow64Process(GetCurrentProcess(), &g_isWow64);
    g_heap = GetProcessHeap();
}
