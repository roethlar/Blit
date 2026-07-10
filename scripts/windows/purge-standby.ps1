# Empty the Windows standby list (the file cache) — the drop_caches
# equivalent for the otp-2w benchmark (scripts/bench_otp2w_baseline.sh
# stages this file to the daemon host and invokes it before every
# timed run). Requires an administrator token; enables
# SeProfileSingleProcessPrivilege, then asks the memory manager to
# purge the standby list (SystemMemoryListInformation / command 4).
$ErrorActionPreference = 'Stop'

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class StandbyPurge
{
    [StructLayout(LayoutKind.Sequential)]
    public struct LUID { public uint LowPart; public int HighPart; }

    [StructLayout(LayoutKind.Sequential)]
    public struct TOKEN_PRIVILEGES { public int Count; public LUID Luid; public int Attr; }

    [DllImport("advapi32.dll", SetLastError = true)]
    public static extern bool OpenProcessToken(IntPtr h, int acc, ref IntPtr tok);

    [DllImport("advapi32.dll", SetLastError = true)]
    public static extern bool LookupPrivilegeValue(string host, string name, ref LUID luid);

    [DllImport("advapi32.dll", SetLastError = true)]
    public static extern bool AdjustTokenPrivileges(IntPtr tok, bool dis, ref TOKEN_PRIVILEGES newst, int len, IntPtr prev, IntPtr rel);

    [DllImport("kernel32.dll")]
    public static extern IntPtr GetCurrentProcess();

    [DllImport("ntdll.dll")]
    public static extern uint NtSetSystemInformation(int infoClass, ref int info, int len);

    public static uint Purge()
    {
        IntPtr tok = IntPtr.Zero;
        LUID luid = new LUID();
        OpenProcessToken(GetCurrentProcess(), 0x20 /*ADJUST*/ | 0x8 /*QUERY*/, ref tok);
        LookupPrivilegeValue(null, "SeProfileSingleProcessPrivilege", ref luid);
        TOKEN_PRIVILEGES tp;
        tp.Count = 1; tp.Luid = luid; tp.Attr = 0x2 /*ENABLED*/;
        AdjustTokenPrivileges(tok, false, ref tp, 0, IntPtr.Zero, IntPtr.Zero);
        int cmd = 4; // MemoryPurgeStandbyList
        return NtSetSystemInformation(80 /*SystemMemoryListInformation*/, ref cmd, 4);
    }
}
"@

$rc = [StandbyPurge]::Purge()
if ($rc -ne 0) { throw "NtSetSystemInformation failed: 0x$($rc.ToString('x'))" }
Write-Output "standby-purged"
