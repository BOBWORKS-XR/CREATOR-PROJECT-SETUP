param(
    [string]$MakeNsis = "$env:LOCALAPPDATA\tauri\NSIS\makensis.exe",
    [switch]$ExpectLegacyPage
)
$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$root = Join-Path $repo ("dist\installer-guard-" + [Guid]::NewGuid().ToString('N'))
[void](New-Item -ItemType Directory -Path $root)
$exe = Join-Path $root 'guard-fixture.exe'
$marker = Join-Path $root 'legacy-page.reached'
$guard = Join-Path $repo 'src-tauri\windows\installer-hooks.nsh'
$node = (Get-Command node.exe -ErrorAction Stop).Source
$fixtureNode = Join-Path $root 'creator-project-setup.exe'
Copy-Item -LiteralPath $node -Destination $fixtureNode
$existing = @(Get-Process -Name creator-project-setup -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Id)
& $MakeNsis /V2 "/DGUARD_FILE=$guard" "/DFIXTURE_EXE=$exe" (Join-Path $PSScriptRoot 'installer-guard-fixture.nsi')
if ($LASTEXITCODE -ne 0) { throw 'Could not compile the no-install NSIS fixture.' }

Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public static class SetupGuardWindows {
    private delegate bool Visit(IntPtr h, IntPtr data);
    [DllImport("user32.dll")] private static extern bool EnumWindows(Visit cb, IntPtr data);
    [DllImport("user32.dll")] private static extern bool EnumChildWindows(IntPtr root, Visit cb, IntPtr data);
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] private static extern int GetWindowText(IntPtr h, StringBuilder text, int length);
    [DllImport("user32.dll")] private static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
    public static string Acknowledge(uint pid) {
        string result = null;
        EnumWindows((h, _) => {
            uint owner; GetWindowThreadProcessId(h, out owner);
            if (owner != pid) return true;
            var texts = new List<string>();
            IntPtr acknowledge = IntPtr.Zero;
            EnumChildWindows(h, (c, d) => {
                var s = new StringBuilder(4096); GetWindowText(c, s, s.Capacity);
                texts.Add(s.ToString());
                if (s.ToString() == "OK") acknowledge = c;
                return true;
            }, IntPtr.Zero);
            var message = string.Join("\n", texts);
            if (message.Contains("Setup will not force-close it.")) {
                result = message;
                if (acknowledge != IntPtr.Zero) PostMessage(acknowledge, 0xF5, IntPtr.Zero, IntPtr.Zero);
            }
            return true;
        }, IntPtr.Zero);
        return result;
    }
}
'@

$psi = [Diagnostics.ProcessStartInfo]::new($fixtureNode)
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
# Keep this owned fake app alive independently of stdin readiness, until the
# harness sends its exact release marker.
$psi.Arguments = '-e "const hold=setInterval(()=>{},1000);let input='''';process.stdin.setEncoding(''utf8'');process.stdin.on(''data'',chunk=>{input+=chunk;if(input.trim()===''exit''){clearInterval(hold);process.exit(0)}});process.stdout.write(''ready\n'')"'
$owned = [Diagnostics.Process]::Start($psi)
$fixtureError = $owned.StandardError.ReadToEndAsync()
$run = $null
$result = [ordered]@{ guardSha256=(Get-FileHash -LiteralPath $guard -Algorithm SHA256).Hash.ToLowerInvariant(); legacyExpected=[bool]$ExpectLegacyPage; passed=$false }
try {
    $ready = $owned.StandardOutput.ReadLineAsync()
    if (-not $ready.Wait(10000) -or $ready.Result -ne 'ready') { throw 'Fixture process did not become ready.' }
    $result.fixturePid = $owned.Id
    if ($owned.HasExited) { throw 'Fixture exited before the installer test could start.' }
    $run = Start-Process -FilePath $exe -PassThru -WindowStyle Hidden
    $dialog = $null
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    while (-not $run.HasExited -and [DateTime]::UtcNow -lt $deadline) {
        $observed = [SetupGuardWindows]::Acknowledge($run.Id)
        if ($observed) { $dialog = $observed }
        Start-Sleep -Milliseconds 100
    }
    if (-not $run.HasExited) { throw "Fixture did not exit; inspect only its PID $($run.Id). No process was killed." }
    $run.WaitForExit()
    $result.exitCode = $run.ExitCode
    $result.legacyPageReached = Test-Path -LiteralPath $marker
    $result.dialog = $dialog
    $result.fixtureSurvived = -not $owned.HasExited
    $result.existingProcessesSurvived = @($existing | Where-Object { -not (Get-Process -Id $_ -ErrorAction SilentlyContinue) }).Count -eq 0
    if (-not $result.fixtureSurvived) { throw 'Fake app exited unexpectedly; installer refusal cannot be evaluated.' }
    if ($ExpectLegacyPage) {
        if (-not $result.legacyPageReached -or $result.exitCode -ne 0) { throw 'Baseline did not reach the legacy page.' }
    } elseif ($result.legacyPageReached -or $result.exitCode -ne 10 -or -not $dialog) {
        throw 'Early refusal was not proven before the legacy page.'
    }
    if (-not $result.fixtureSurvived -or -not $result.existingProcessesSurvived) { throw 'A protected process exited during the test.' }
    $result.passed = $true
} finally {
    if (-not $owned.HasExited) { $owned.StandardInput.WriteLine('exit'); $owned.StandardInput.Close() }
    if (-not $owned.WaitForExit(10000)) { throw "Owned fixture Node did not exit cooperatively: $($owned.Id)" }
    $result.fixtureExitCode = $owned.ExitCode
    $result.fixtureStderr = if ($fixtureError.Wait(1000)) { $fixtureError.Result } else { 'stderr capture did not complete' }
    $owned.Dispose()
    if ($run -and $run.HasExited) { $run.Dispose() }
    $result | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $root 'report.json') -Encoding UTF8
    Write-Output (Join-Path $root 'report.json')
}
