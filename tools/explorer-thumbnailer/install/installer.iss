[Setup]
AppName="resvg Explorer Extension"
AppVersion="0.45.0"
VersionInfoVersion="0.0.45.0"
AppVerName="resvg Explorer Extension 0.45.0"
AppPublisher="The Resvg Authors"
AppPublisherURL=https://github.com/linebender/resvg
DefaultDirName="{pf}\resvg Explorer Extension"
Compression=lzma
SolidCompression=yes
ChangesAssociations=yes
DisableDirPage=yes
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
OutputBaseFilename="resvg-explorer-extension"
OutputDir=.

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"; LicenseFile: "..\LICENSE-SUMMARY.txt"

[Files]
Source: "..\target\release\server.dll"; DestDir: "{app}"
Source: "..\LICENSE-APACHE"; DestDir: "{app}";
Source: "..\LICENSE-MIT"; DestDir: "{app}";

[Registry]
Root: HKCR; Subkey: "CLSID\{{4432C229-DFD0-4B18-8C4D-F58932AF6105}"; Flags: uninsdeletekey
Root: HKCR; Subkey: "CLSID\{{4432C229-DFD0-4B18-8C4D-F58932AF6105}"; ValueType: string; ValueData: "ThumbnailProvider"
Root: HKCR; Subkey: "CLSID\{{4432C229-DFD0-4B18-8C4D-F58932AF6105}\InprocServer32"; ValueType: string; ValueData: "{app}\server.dll"
Root: HKCR; Subkey: ".svg\shellex"; Flags: uninsdeletekeyifempty
Root: HKCR; Subkey: ".svg\shellex\{{E357FCCD-A995-4576-B01F-234630154E96}"; Flags: uninsdeletekey
Root: HKCR; Subkey: ".svg\shellex\{{E357FCCD-A995-4576-B01F-234630154E96}"; ValueType: string; ValueData: "{{4432C229-DFD0-4B18-8C4D-F58932AF6105}"
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Services\EventLog\Application\resvg Thumbnailer"; Flags: uninsdeletekey
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Services\EventLog\Application\resvg Thumbnailer"; ValueType: string; ValueName: "EventMessageFile"; ValueData: "{app}\server.dll"
