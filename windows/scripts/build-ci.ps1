$ErrorActionPreference = "Stop"

function Invoke-BoundedBuild([string] $Name, [string[]] $Extra, [int] $TimeoutSeconds) {
    $stdout = Join-Path $env:RUNNER_TEMP "$Name.stdout.log"
    $stderr = Join-Path $env:RUNNER_TEMP "$Name.stderr.log"
    $arguments = @("build", "windows/DitherKit.Gallery/DitherKit.Gallery.csproj", "--configuration", "Release", "-p:Platform=x64", "-p:UseSharedCompilation=false", "--verbosity", "normal") + $Extra
    Write-Host "Starting $Name build"
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $process = Start-Process dotnet -ArgumentList $arguments -RedirectStandardOutput $stdout -RedirectStandardError $stderr -NoNewWindow -PassThru
    while (-not $process.WaitForExit(15000)) {
        Write-Host "$Name elapsed: $([int] $timer.Elapsed.TotalSeconds)s"
        Get-Process -Name dotnet,csc,VBCSCompiler -ErrorAction SilentlyContinue |
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

# This first build isolates C#/XAML and packaging from FXC. It is not a shader
# test: runtime compilation is never used as a substitute for the native gate.
Invoke-BoundedBuild "winui-managed-isolation" @("-p:DefineConstants=DITHER_RUNTIME_SHADER") 180
# Rebuild with normal symbols so cached runtime-only descriptors cannot pass.
Invoke-BoundedBuild "winui-native-shader" @("--no-restore", "--no-incremental") 240
