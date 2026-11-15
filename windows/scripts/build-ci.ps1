$ErrorActionPreference = "Stop"

# WinUI PRI/XAML tasks are installed with Visual Studio, not with the dotnet CLI.
# setup-msbuild adds the full-framework host and its task directory to PATH.
$msbuild = (Get-Command MSBuild.exe -ErrorAction Stop).Source
Write-Host "Using Visual Studio MSBuild: $msbuild"

function Invoke-BoundedBuild([string] $Name, [string[]] $Extra, [int] $TimeoutSeconds,
    [string] $Project = "windows/DitherKit.Gallery/DitherKit.Gallery.csproj") {
    $stdout = Join-Path $env:RUNNER_TEMP "$Name.stdout.log"
    $stderr = Join-Path $env:RUNNER_TEMP "$Name.stderr.log"
    $arguments = @($Project, "-p:Configuration=Release", "-p:Platform=x64", "-p:UseSharedCompilation=false", "-verbosity:normal", "-nologo") + $Extra
    Write-Host "Starting $Name build"
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process $msbuild -ArgumentList $arguments -RedirectStandardOutput $stdout -RedirectStandardError $stderr -NoNewWindow -PassThru
    while (-not $process.WaitForExit(15000)) {
        Write-Host "$Name elapsed: $([int] $timer.Elapsed.TotalSeconds)s"
        Get-Process -Name MSBuild,dotnet,csc,VBCSCompiler -ErrorAction SilentlyContinue |
            Select-Object Id,ProcessName,CPU,WorkingSet64 | Format-Table | Out-String | Write-Host
        if ($timer.Elapsed.TotalSeconds -gt $TimeoutSeconds) {
            $process.Kill($true)
            $process.WaitForExit()
            Get-Content $stdout
            Get-Content $stderr
            throw "$Name exceeded $TimeoutSeconds seconds"
        }
    }
    Get-Content $stdout
    Get-Content $stderr
    Write-Host "$Name completed in $([int] $timer.Elapsed.TotalSeconds)s with exit $($process.ExitCode)"
    if ($process.ExitCode -ne 0) { throw "$Name failed with exit $($process.ExitCode)" }
}

# This first build isolates C#/XAML and PRI generation from FXC. It is not a
# shader test: runtime compilation never substitutes for the native gate.
Invoke-BoundedBuild "winui-managed-isolation" @("-restore", "-p:DefineConstants=DITHER_RUNTIME_SHADER") 180
# Rebuild with normal symbols so cached runtime-only descriptors cannot pass.
Invoke-BoundedBuild "winui-native-shader" @("-target:Rebuild") 240
# Build tests with the same VS host; dotnet test only runs the resulting assembly.
Invoke-BoundedBuild "winui-golden-build" @("-restore") 180 "windows/DitherKit.WinUI.Tests/DitherKit.WinUI.Tests.csproj"
