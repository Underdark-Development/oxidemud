# OxideMUD Windows Installer Script
# Installs binaries, configures PATH environment variable, handles content upgrades, and sets up a startup task.

# Force UTF-8 output
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

param (
    [string]$InstallDir = "$env:USERPROFILE\oxide",
    [int]$McpPort = 5000,
    [switch]$InstallService = $false,  # OPT-IN scheduled task
    [switch]$NoSpade = $false,
    [switch]$NonInteractive = $false
)

Write-Host "=== OxideMUD Windows Installer ===" -ForegroundColor Blue

# 1. Verify environment and read version
if (!(Test-Path ".version")) {
    Write-Host "Error: .version file not found. Run this installer from the unpacked archive directory." -ForegroundColor Red
    Exit 1
}
$VERSION = Get-Content ".version" -Raw
$VERSION = $VERSION.Trim()
Write-Host "Installing OxideMUD version: v$VERSION" -ForegroundColor Green

if ($NonInteractive -eq $false) {
    # Interactive Prompts
    $user_dir = Read-Host "Enter installation path [$InstallDir]"
    if (![string]::IsNullOrWhiteSpace($user_dir)) {
        $InstallDir = $user_dir
    }

    $user_mcp = Read-Host "Enter MCP server listen port [$McpPort]"
    if (![string]::IsNullOrWhiteSpace($user_mcp) -and $user_mcp -match '^\d+$') {
        $McpPort = [int]$user_mcp
    }

    # Ask for Scheduled Task background task (opt-in)
    $opt_service = Read-Host "Do you want to install a Scheduled Task to run the server in the background at startup? [y/N]"
    if ($opt_service -eq "y" -or $opt_service -eq "yes") {
        $InstallService = $true
    }
}

# Confirming parameters
Write-Host "`nParameters:" -ForegroundColor Green
Write-Host "  Installation Directory: $InstallDir" -ForegroundColor Yellow
Write-Host "  Install Spade Editor:   $($NoSpade -eq $false)" -ForegroundColor Yellow
Write-Host "  MCP Server Port:        $McpPort" -ForegroundColor Yellow
Write-Host "  Install Startup Task:   $($InstallService -eq $true)" -ForegroundColor Yellow
Write-Host ""

# 2. Create target directory structures
Write-Host "Setting up directory structure..." -ForegroundColor Green
New-Item -ItemType Directory -Force -Path "$InstallDir\bin" | Out-Null
New-Item -ItemType Directory -Force -Path "$InstallDir\data" | Out-Null

# 3. Copy Binaries
Write-Host "Installing binaries..." -ForegroundColor Green
Copy-Item -Force -Path "bin\oxide-server.exe" -Destination "$InstallDir\bin\"
Copy-Item -Force -Path "bin\oxide-mcp.exe" -Destination "$InstallDir\bin\"

if ($NoSpade -eq $false) {
    Copy-Item -Force -Path "bin\spade.exe" -Destination "$InstallDir\bin\"
    
    # 4. Add Binaries to PATH (so spade and oxide-mcp are globally executable)
    $BIN_DIR = "$InstallDir\bin"
    Write-Host "Configuring PATH environment variable..." -ForegroundColor Green
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -split ';' -notcontains $BIN_DIR) {
        [Environment]::SetEnvironmentVariable("Path", "$UserPath;$BIN_DIR", "User")
        Write-Host "  Successfully added $BIN_DIR to User PATH." -ForegroundColor Green
        Write-Host "  Note: You may need to restart your terminal/IDE for this to take effect." -ForegroundColor Yellow
    } else {
        Write-Host "  $BIN_DIR is already in User PATH." -ForegroundColor Yellow
    }
}

# 5. Handle Content Upgrades
if (Test-Path "$InstallDir\content") {
    # Upgrade scenario
    Write-Host "Upgrade detected. Preserving existing content folder." -ForegroundColor Yellow
    
    # Backup active SQLite database before upgrade schema migrations trigger
    $DbFile = "$InstallDir\data\oxide.db"
    if (Test-Path $DbFile) {
        $BackupTime = (Get-Date).ToString("yyyyMMdd_HHmmss")
        $BackupDir = "$InstallDir\data\backups"
        if (-not (Test-Path $BackupDir)) {
            New-Item -ItemType Directory -Force -Path $BackupDir | Out-Null
        }
        Copy-Item -Force -Path $DbFile -Destination "$BackupDir\oxide.db.pre-upgrade-$BackupTime"
        Write-Host "  Backed up active database to: $BackupDir\oxide.db.pre-upgrade-$BackupTime" -ForegroundColor Green
    }
    
    # Store old version if readable
    $OLD_VERSION = "unknown"
    if (Test-Path "$InstallDir\.version") {
        $OLD_VERSION = (Get-Content "$InstallDir\.version" -Raw).Trim()
    }
    Write-Host "  Upgrading from v$OLD_VERSION to v$VERSION" -ForegroundColor Green
    
    # Place new baseline templates in content.default/
    if (Test-Path "$InstallDir\content.default") {
        Remove-Item -Recurse -Force -Path "$InstallDir\content.default"
    }
    Copy-Item -Recurse -Force -Path "content" -Destination "$InstallDir\content.default"
    Write-Host "  Placed new default templates in $InstallDir\content.default\ for reference." -ForegroundColor Green
} else {
    # Fresh Install scenario
    Write-Host "Fresh install detected. Copying default templates..." -ForegroundColor Green
    Copy-Item -Recurse -Force -Path "content" -Destination "$InstallDir\content"
    Write-Host "  Installed templates to: $InstallDir\content\" -ForegroundColor Green
}

# Write target version metadata
$VERSION | Out-File -FilePath "$InstallDir\.version" -NoNewline -Encoding utf8

# 6. Configure Background Task for server (Scheduled Task)
if ($InstallService -eq $true) {
    Write-Host "`nSetting up background Scheduled Task for the MUD server..." -ForegroundColor Green

    $TaskName = "OxideMUDServer"
    $Action = New-ScheduledTaskAction -Execute "$InstallDir\bin\oxide-server.exe" -Argument "--config-path $InstallDir\content\server.toml --db-path $InstallDir\data\oxide.db" -WorkingDirectory "$InstallDir"
    $Trigger = New-ScheduledTaskTrigger -AtStartup
    $Settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)

    # Check if task already exists, unregistered first
    if (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue) {
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false | Out-Null
        Write-Host "  Replaced existing scheduled task: $TaskName" -ForegroundColor Yellow
    }

    # Register the task (run under current user context on startup)
    $Principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType S4U
    $Task = New-ScheduledTask -Action $Action -Trigger $Trigger -Settings $Settings -Principal $Principal

    try {
        Register-ScheduledTask -TaskName $TaskName -InputObject $Task -ErrorAction Stop | Out-Null
        Write-Host "  Successfully created startup task: $TaskName" -ForegroundColor Green
        Write-Host "  This task will run the server in the background whenever the computer boots." -ForegroundColor Yellow
        Write-Host "  To start the server now, run:  Start-ScheduledTask -TaskName $TaskName" -ForegroundColor Green
        Write-Host "  To stop the server, run:       Stop-ScheduledTask -TaskName $TaskName" -ForegroundColor Green
    } catch {
        Write-Host "  Could not register scheduled task: $_" -ForegroundColor Red
        Write-Host "  You can still start the server manually using: $InstallDir\bin\oxide-server.exe" -ForegroundColor Yellow
    }
} else {
    Write-Host "`nManual Launch Commands (startup task not installed):" -ForegroundColor Yellow
    Write-Host "  To start the game server manually, run:" -ForegroundColor Green
    Write-Host "    $InstallDir\bin\oxide-server.exe --config-path $InstallDir\content\server.toml --db-path $InstallDir\data\oxide.db" -ForegroundColor Yellow
}

Write-Host "`nInstallation Complete!" -ForegroundColor Green
