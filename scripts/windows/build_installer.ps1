# Dream Launcher Windows installer build script.
# In this script we use NSIS for creating the installer.

param(
    [string]$Version = "0.1.0",
    [string]$BuildDir = "target/release",
    [string]$OutputDir = "dist",
    [string]$Architecture = "x64",
    [switch]$Clean = $false
)

# Set error action preference.
$ErrorActionPreference = "Stop"

# Colors.
function Write-ColorOutput($ForegroundColor) {
    $fc = $host.UI.RawUI.ForegroundColor
    $host.UI.RawUI.ForegroundColor = $ForegroundColor
    if ($args) {
        Write-Output $args
    } else {
        $input | Write-Output
    }
    $host.UI.RawUI.ForegroundColor = $fc
}

function Write-Info($message) {
    Write-ColorOutput Cyan "[INFO] $message"
}

function Write-Success($message) {
    Write-ColorOutput Green "[SUCCESS] $message"
}

function Write-Error($message) {
    Write-ColorOutput Red "[ERROR] $message"
}

function Write-Warning($message) {
    Write-ColorOutput Yellow "[WARNING] $message"
}

# Check if NSIS is installed.
function Test-NSISInstalled {
    $nsisPath = Get-Command "makensis.exe" -ErrorAction SilentlyContinue
    if (-not $nsisPath) {
        # Try common installation paths
        $commonPaths = @(
            "${env:ProgramFiles}\NSIS\makensis.exe",
            "${env:ProgramFiles(x86)}\NSIS\makensis.exe",
            "C:\Program Files\NSIS\makensis.exe",
            "C:\Program Files (x86)\NSIS\makensis.exe"
        )
        
        foreach ($path in $commonPaths) {
            if (Test-Path $path) {
                return $path
            }
        }
        return $null
    }
    return $nsisPath.Source
}

# Main script.
try {
    Write-Info "Starting Dream Launcher Windows installer build..."
    
    # Check prerequisites
    Write-Info "Checking prerequisites..."
    
    $nsisPath = Test-NSISInstalled
    if (-not $nsisPath) {
        Write-Error "NSIS is not installed or not found in PATH."
        Write-Info "Please install NSIS from: https://nsis.sourceforge.io/Download"
        Write-Info "Or install via Chocolatey: choco install nsis"
        exit 1
    }
    Write-Success "NSIS found at: $nsisPath"
    
    # Resolve paths
    $scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    $projectRoot = Resolve-Path (Join-Path $scriptDir "../..")
    
    # Adjust build path for ARM64
    if ($Architecture -eq "ARM64") {
        $buildPath = Join-Path $projectRoot "target/aarch64-pc-windows-msvc/release"
    } else {
        $buildPath = Join-Path $projectRoot $BuildDir
    }
    
    $outputPath = Join-Path $projectRoot $OutputDir
    $nsisScript = Join-Path $scriptDir "installer.nsi"
    
    Write-Info "Project root: $projectRoot"
    Write-Info "Build directory: $buildPath"
    Write-Info "Output directory: $outputPath"
    Write-Info "Architecture: $Architecture"
    
    # Check if executable exists
    $exePath = Join-Path $buildPath "DreamLauncher.exe"
    if (-not (Test-Path $exePath)) {
        Write-Error "Executable not found at: $exePath"
        exit 1
    }
    Write-Success "Executable found: $exePath"
    
    # Create output directory
    if (-not (Test-Path $outputPath)) {
        New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
        Write-Info "Created output directory: $outputPath"
    }
    
    # Clean output directory if requested
    if ($Clean) {
        Write-Info "Cleaning output directory..."
        Get-ChildItem $outputPath -Filter "*.exe" | Remove-Item -Force
    }
    
    # Check for LICENSE file
    $licensePath = Join-Path $projectRoot "LICENSE"
    
    # Build installer
    Write-Info "Building installer with NSIS..."
    
    # Set installer name based on architecture
    $installerName = if ($Architecture -eq "ARM64") { "Dream Launcher Setup ARM64.exe" } else { "Dream Launcher Setup.exe" }
    
    $nsisArgs = @(
        "/DAPP_VERSION=$Version",
        "/DOUTPUT_DIR=$outputPath",
        "/DARCHITECTURE=$Architecture",
        "/DINSTALLER_NAME=$installerName",
        "/DEXE_PATH=$exePath",
        $nsisScript
    )
    
    $process = Start-Process -FilePath $nsisPath -ArgumentList $nsisArgs -Wait -PassThru -NoNewWindow
    
    if ($process.ExitCode -eq 0) {
        $installerPath = Join-Path $outputPath $installerName
        if (Test-Path $installerPath) {
            $fileSize = [math]::Round((Get-Item $installerPath).Length / 1MB, 2)
            Write-Success "Installer built successfully!"
            Write-Success "Location: $installerPath"
            Write-Success "Size: $fileSize MB"
        } else {
            Write-Error "Installer was not created at expected location: $installerPath"
            exit 1
        }
    } else {
        Write-Error "NSIS compilation failed with exit code: $($process.ExitCode)"
        exit 1
    }
    
    Write-Success "Build completed successfully!"
    
} catch {
    Write-Error "An error occurred: $($_.Exception.Message)"
    Write-Error $_.ScriptStackTrace
    exit 1
}
