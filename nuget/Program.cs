using System.Diagnostics;
using System.Runtime.InteropServices;

return Run();

static int Run()
{
    string? rid = ResolveRid();
    if (rid is null)
    {
        Console.Error.WriteLine(
            $"klint: unsupported platform {RuntimeInformation.OSDescription} / {RuntimeInformation.ProcessArchitecture}");
        return 1;
    }

    string binary = Path.Combine(AppContext.BaseDirectory, "binaries", rid, BinaryName());
    if (!File.Exists(binary))
    {
        Console.Error.WriteLine($"klint: bundled native binary is missing: {binary}");
        return 1;
    }

    EnsureExecutable(binary);

    var startInfo = new ProcessStartInfo(binary) { UseShellExecute = false };
    foreach (string arg in Environment.GetCommandLineArgs().Skip(1))
    {
        startInfo.ArgumentList.Add(arg);
    }

    using var process = Process.Start(startInfo);
    if (process is null)
    {
        Console.Error.WriteLine($"klint: failed to launch native binary: {binary}");
        return 1;
    }

    process.WaitForExit();
    return process.ExitCode;
}

// Maps the host to the .NET RID folder the binaries are staged under.
static string? ResolveRid()
{
    string? os = RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "win"
        : RuntimeInformation.IsOSPlatform(OSPlatform.Linux) ? "linux"
        : RuntimeInformation.IsOSPlatform(OSPlatform.OSX) ? "osx"
        : null;

    string? arch = RuntimeInformation.ProcessArchitecture switch
    {
        Architecture.X64 => "x64",
        Architecture.Arm64 => "arm64",
        _ => null,
    };

    return os is null || arch is null ? null : $"{os}-{arch}";
}

static string BinaryName() =>
    RuntimeInformation.IsOSPlatform(OSPlatform.Windows) ? "klint-rs.exe" : "klint-rs";

// NuGet extraction does not preserve the executable bit, so restore it before launch.
static void EnsureExecutable(string path)
{
    if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
    {
        return;
    }

    UnixFileMode current = File.GetUnixFileMode(path);
    UnixFileMode executable = current
        | UnixFileMode.UserExecute
        | UnixFileMode.GroupExecute
        | UnixFileMode.OtherExecute;

    if (current != executable)
    {
        File.SetUnixFileMode(path, executable);
    }
}
