; Dream Launcher NSIS installer script.

!define APP_NAME "Dream Launcher"
!ifndef APP_VERSION
  !define APP_VERSION "0.1.0"
!endif
!define APP_PUBLISHER "Frogdream Studios"
!define APP_URL "https://github.com/FrogdreamStudios/launcher"
!define APP_EXECUTABLE "DreamLauncher.exe"
!define APP_REGKEY "Software\\${APP_PUBLISHER}\\${APP_NAME}"
!define UNINSTALL_REGKEY "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\${APP_NAME}"

; UI.
!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "WinVer.nsh"
!include "x64.nsh"

; General settings.
Name "${APP_NAME}"
OutFile "Dream Launcher Setup.exe"
InstallDir "$PROGRAMFILES64\\${APP_NAME}"
InstallDirRegKey HKLM "${APP_REGKEY}" "InstallPath"
RequestExecutionLevel admin
ShowInstDetails show
ShowUnInstDetails show

; Compression.
SetCompressor /SOLID lzma
SetCompressorDictSize 32

; Version Information.
VIProductVersion "0.1.0.0"
VIAddVersionKey "ProductName" "${APP_NAME}"
VIAddVersionKey "ProductVersion" "${APP_VERSION}"
VIAddVersionKey "CompanyName" "${APP_PUBLISHER}"
VIAddVersionKey "FileDescription" "${APP_NAME} Setup"
VIAddVersionKey "FileVersion" "${APP_VERSION}"
VIAddVersionKey "LegalCopyright" "© ${APP_PUBLISHER}"

; UI Configuration.
!define MUI_ABORTWARNING
!define MUI_ICON "${NSISDIR}\\Contrib\\Graphics\\Icons\\modern-install.ico"
!define MUI_UNICON "${NSISDIR}\\Contrib\\Graphics\\Icons\\modern-uninstall.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_RIGHT
!define MUI_HEADERIMAGE_BITMAP "${NSISDIR}\\Contrib\\Graphics\\Header\\nsis3-metro.bmp"
!define MUI_HEADERIMAGE_UNBITMAP "${NSISDIR}\\Contrib\\Graphics\\Header\\nsis3-metro.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP "${NSISDIR}\\Contrib\\Graphics\\Wizard\\nsis3-metro.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "${NSISDIR}\\Contrib\\Graphics\\Wizard\\nsis3-metro.bmp"

; Welcome page.
!define MUI_WELCOMEPAGE_TITLE "Welcome to ${APP_NAME} Setup"
!define MUI_WELCOMEPAGE_TEXT "This wizard will guide you through the installation of ${APP_NAME}.\r\n\r\n${APP_NAME} is a powerful and lightweight Minecraft launcher.\r\n\r\nClick Next to continue."
!insertmacro MUI_PAGE_WELCOME

; License page.
!insertmacro MUI_PAGE_LICENSE "LICENSE"

; Components page.
!insertmacro MUI_PAGE_COMPONENTS

; Directory page.
!insertmacro MUI_PAGE_DIRECTORY

; Start menu page.
Var StartMenuFolder
!define MUI_STARTMENUPAGE_REGISTRY_ROOT "HKLM"
!define MUI_STARTMENUPAGE_REGISTRY_KEY "${APP_REGKEY}"
!define MUI_STARTMENUPAGE_REGISTRY_VALUENAME "StartMenuFolder"
!insertmacro MUI_PAGE_STARTMENU Application $StartMenuFolder

; Installation page.
!insertmacro MUI_PAGE_INSTFILES

; Finish page.
!define MUI_FINISHPAGE_RUN "$INSTDIR\\${APP_EXECUTABLE}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ${APP_NAME}"
!define MUI_FINISHPAGE_LINK "Visit our GitHub"
!define MUI_FINISHPAGE_LINK_LOCATION "${APP_URL}"
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages.
!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

; Languages.
!insertmacro MUI_LANGUAGE "English"

; Installation types.
InstType "Full"
InstType "Minimal"

; Sections.
Section "!${APP_NAME} (required)" SecMain
  SectionIn RO 1 2
  
  ; Set output path to the installation directory
  SetOutPath "$INSTDIR"
  
  ; Copy main executable
  File "..\\..\\target\\release\\${APP_EXECUTABLE}"
  
  ; Copy assets if they exist
  SetOutPath "$INSTDIR\\assets"
  File /r /x ".git*" "..\\..\\assets\\*"
  
  ; Store installation folder
  WriteRegStr HKLM "${APP_REGKEY}" "InstallPath" "$INSTDIR"
  WriteRegStr HKLM "${APP_REGKEY}" "Version" "${APP_VERSION}"
  
  ; Create uninstaller
  WriteUninstaller "$INSTDIR\\Uninstall.exe"
  
  ; Add uninstall information to Add/Remove Programs
  WriteRegStr HKLM "${UNINSTALL_REGKEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKLM "${UNINSTALL_REGKEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "${UNINSTALL_REGKEY}" "Publisher" "${APP_PUBLISHER}"
  WriteRegStr HKLM "${UNINSTALL_REGKEY}" "URLInfoAbout" "${APP_URL}"
  WriteRegStr HKLM "${UNINSTALL_REGKEY}" "DisplayIcon" "$INSTDIR\\${APP_EXECUTABLE}"
  WriteRegStr HKLM "${UNINSTALL_REGKEY}" "UninstallString" "$INSTDIR\\Uninstall.exe"
  WriteRegStr HKLM "${UNINSTALL_REGKEY}" "QuietUninstallString" "$INSTDIR\\Uninstall.exe /S"
  WriteRegDWORD HKLM "${UNINSTALL_REGKEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTALL_REGKEY}" "NoRepair" 1
  
  ; Calculate and store the size
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "${UNINSTALL_REGKEY}" "EstimatedSize" "$0"
SectionEnd

Section "Desktop Shortcut" SecDesktop
  SectionIn 1
  CreateShortcut "$DESKTOP\\${APP_NAME}.lnk" "$INSTDIR\\${APP_EXECUTABLE}" "" "$INSTDIR\\${APP_EXECUTABLE}" 0
SectionEnd

Section "Start Menu Shortcuts" SecStartMenu
  SectionIn 1 2
  
  !insertmacro MUI_STARTMENU_WRITE_BEGIN Application
  
  CreateDirectory "$SMPROGRAMS\\$StartMenuFolder"
  CreateShortcut "$SMPROGRAMS\\$StartMenuFolder\\${APP_NAME}.lnk" "$INSTDIR\\${APP_EXECUTABLE}" "" "$INSTDIR\\${APP_EXECUTABLE}" 0
  CreateShortcut "$SMPROGRAMS\\$StartMenuFolder\\Uninstall ${APP_NAME}.lnk" "$INSTDIR\\Uninstall.exe" "" "$INSTDIR\\Uninstall.exe" 0
  
  !insertmacro MUI_STARTMENU_WRITE_END
SectionEnd

Section "File Associations" SecFileAssoc
  SectionIn 1
  
  ; Register .dreamlauncher file extension
  WriteRegStr HKCR ".dreamlauncher" "" "DreamLauncher.Instance"
  WriteRegStr HKCR "DreamLauncher.Instance" "" "Dream Launcher Instance"
  WriteRegStr HKCR "DreamLauncher.Instance\\DefaultIcon" "" "$INSTDIR\\${APP_EXECUTABLE},0"
  WriteRegStr HKCR "DreamLauncher.Instance\\shell\\open\\command" "" '"$INSTDIR\\${APP_EXECUTABLE}" "%1"'
SectionEnd

; Section descriptions.
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecMain} "The core ${APP_NAME} application files. This component is required."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} "Creates a shortcut on the desktop for easy access to ${APP_NAME}."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecStartMenu} "Creates shortcuts in the Start Menu."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecFileAssoc} "Associates .dreamlauncher files with ${APP_NAME}."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; Installation functions.
Function .onInit
  ; Check if running on 64-bit Windows
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "${APP_NAME} requires 64-bit Windows."
    Abort
  ${EndIf}
  
  ; Check Windows version (Windows 10 or later)
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_OK|MB_ICONSTOP "${APP_NAME} requires Windows 10 or later."
    Abort
  ${EndIf}
  
  ; Check if already installed
  ReadRegStr $R0 HKLM "${APP_REGKEY}" "InstallPath"
  ${If} $R0 != ""
    MessageBox MB_YESNO|MB_ICONQUESTION "${APP_NAME} is already installed. Do you want to reinstall it?" IDYES +2
    Abort
  ${EndIf}
FunctionEnd

Function .onInstSuccess
  ; Refresh shell icons
  System::Call 'shell32.dll::SHChangeNotify(l, l, p, p) v (0x08000000, 0, 0, 0)'
FunctionEnd

; Uninstaller section.
Section "Uninstall"
  ; Remove files
  Delete "$INSTDIR\\${APP_EXECUTABLE}"
  Delete "$INSTDIR\\Uninstall.exe"
  
  ; Remove assets
  RMDir /r "$INSTDIR\\assets"
  
  ; Remove shortcuts
  Delete "$DESKTOP\\${APP_NAME}.lnk"
  
  ; Remove Start Menu shortcuts
  !insertmacro MUI_STARTMENU_GETFOLDER Application $StartMenuFolder
  Delete "$SMPROGRAMS\\$StartMenuFolder\\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\\$StartMenuFolder\\Uninstall ${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\\$StartMenuFolder"
  
  ; Remove file associations
  DeleteRegKey HKCR ".dreamlauncher"
  DeleteRegKey HKCR "DreamLauncher.Instance"
  
  ; Remove registry keys
  DeleteRegKey HKLM "${UNINSTALL_REGKEY}"
  DeleteRegKey HKLM "${APP_REGKEY}"
  
  ; Remove installation directory if empty
  RMDir "$INSTDIR"
  
  ; Refresh shell icons
  System::Call 'shell32.dll::SHChangeNotify(l, l, p, p) v (0x08000000, 0, 0, 0)'
SectionEnd

; Uninstaller functions.
Function un.onInit
  MessageBox MB_YESNO|MB_ICONQUESTION "Are you sure you want to completely remove ${APP_NAME} and all of its components?" IDYES +2
  Abort
FunctionEnd
