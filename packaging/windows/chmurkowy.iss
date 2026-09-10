; Instalator Chmurkowego Launchera dla Windowsa.
;
; Instaluje "dla użytkownika", a nie dla całego komputera. To nie jest
; drobiazg: launcher aktualizuje się sam, podmieniając własny plik, więc
; musi leżeć w katalogu, do którego ma prawo zapisu bez pytania o hasło
; administratora. W Program Files każda poprawka wymagałaby ponownej
; instalacji z podniesionymi uprawnieniami.
;
; Wersję podaje się przy budowaniu:
;   ISCC.exe /DWersja=0.4.14 chmurkowy.iss

#ifndef Wersja
  #define Wersja "0.0.0"
#endif

[Setup]
; Staly identyfikator — po nim Windows poznaje, ze to ta sama aplikacja
; przy kolejnych instalacjach. Nie zmieniac.
AppId={{7B3C1E42-9D5A-4F18-AC77-2E6B0D9F4A31}
AppName=Chmurkowy Launcher
AppVersion={#Wersja}
AppPublisher=Chmurkowy Serwer
DefaultDirName={autopf}\ChmurkowyLauncher
DefaultGroupName=Chmurkowy Launcher
DisableProgramGroupPage=yes
DisableDirPage=auto
PrivilegesRequired=lowest
OutputDir=.
OutputBaseFilename=ChmurkowyLauncher-setup
SetupIconFile=ikona.ico
UninstallDisplayIcon={app}\ChmurkowyLauncher.exe
UninstallDisplayName=Chmurkowy Launcher
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; Nie zostawiamy dzialajacego launchera z podmienionym plikiem pod spodem.
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "polski"; MessagesFile: "compiler:Languages\Polish.isl"

[Tasks]
Name: "pulpit"; Description: "Utwórz skrót na pulpicie"; GroupDescription: "Skróty:"

[Files]
Source: "ChmurkowyLauncher.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Chmurkowy Launcher"; Filename: "{app}\ChmurkowyLauncher.exe"
Name: "{userdesktop}\Chmurkowy Launcher"; Filename: "{app}\ChmurkowyLauncher.exe"; Tasks: pulpit
Name: "{group}\Odinstaluj Chmurkowy Launcher"; Filename: "{uninstallexe}"

[Run]
Filename: "{app}\ChmurkowyLauncher.exe"; Description: "Uruchom Chmurkowy Launcher"; Flags: nowait postinstall skipifsilent

; Gra, paczka modow i swiaty leza w %LOCALAPPDATA%\ChmurkowyLauncher i celowo
; NIE sa tu wymienione. Odinstalowanie ma usunac program, a nie swiaty gracza.
; Kto chce posprzatac do konca, kasuje ten katalog recznie.
