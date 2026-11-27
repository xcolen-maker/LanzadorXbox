; Instalador de Lanzador Xbox (Inno Setup 6).
; Se genera con:  ISCC.exe installer\LanzadorXbox.iss   →   dist\LanzadorXbox-Setup-<versión>.exe
; Se instala para el usuario, sin pedir permisos de administrador.

#define AppName "Lanzador Xbox"
#define AppVersion "0.1.0"
#define AppExe "Lanzador-Xbox.exe"

[Setup]
AppId={{14B98642-F64A-41DB-97D5-781D93091A5D}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=xcolen-maker
DefaultDirName={localappdata}\Programs\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=LanzadorXbox-Setup-{#AppVersion}
SetupIconFile=..\icon\app.ico
UninstallDisplayIcon={app}\{#AppExe}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "desktopicon"; Description: "Crear un acceso directo en el escritorio"; GroupDescription: "Accesos directos:"

[Files]
Source: "..\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\imagenes\*"; DestDir: "{app}\imagenes"; Flags: ignoreversion skipifsourcedoesntexist

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{group}\Desinstalar {#AppName}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExe}"; Description: "Abrir {#AppName}"; Flags: nowait postinstall skipifsilent
