; installer-hooks.nsh - Uninstall keeps the library unless "delete app data" is ticked (spec 13.6).
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    ; Export first: copy the database to Documents before removing MooDeX v2 data.
    CreateDirectory "$DOCUMENTS\MooDeX backup"
    CopyFiles /SILENT "$LOCALAPPDATA\MooDeX\v2\moodex.db" "$DOCUMENTS\MooDeX backup\moodex.db"
    ; Only our own v2 folder - never the shared %LOCALAPPDATA%\MooDeX root.
    RMDir /r "$LOCALAPPDATA\MooDeX\v2"
  ${EndIf}
!macroend
