; installer-hooks.nsh - Moves an old per-user install to Program Files; uninstall keeps the library unless asked (spec 13.6).

; Up to 2.0.8 MooDeX installed per user into %LOCALAPPDATA%\MooDeX. That folder also holds the
; library (v2\) and files of other apps, so only the three files our installer put there are
; removed, and only shortcuts that point at the old exe are deleted.
!macro NSIS_HOOK_POSTINSTALL
  ; This installer runs per machine; the old install lives in the signed-in user's profile.
  SetShellVarContext current
  StrCpy $R8 "$LOCALAPPDATA\${PRODUCTNAME}"
  ReadRegStr $R7 HKCU "${UNINSTKEY}" "UninstallString"
  ${If} $R7 != ""
  ${AndIf} $R8 != $INSTDIR
    DetailPrint "Removing the previous per-user install in $R8"
    ; The updater closes MooDeX as it starts this installer; give the old exe time to unlock.
    StrCpy $R9 0
    ${Do}
      Delete "$R8\${MAINBINARYNAME}.exe"
      ${IfNot} ${FileExists} "$R8\${MAINBINARYNAME}.exe"
        ${Break}
      ${EndIf}
      IntOp $R9 $R9 + 1
      ${If} $R9 >= 20
        Delete /REBOOTOK "$R8\${MAINBINARYNAME}.exe"
        ${Break}
      ${EndIf}
      Sleep 500
    ${Loop}
    Delete "$R8\WebView2Loader.dll"
    Delete "$R8\uninstall.exe"
    ; Removed only if empty; it normally keeps the v2\ library folder.
    RMDir "$R8"

    !insertmacro IsShortcutTarget "$DESKTOP\${PRODUCTNAME}.lnk" "$R8\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      Delete "$DESKTOP\${PRODUCTNAME}.lnk"
    ${EndIf}
    !insertmacro IsShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}.lnk" "$R8\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      Delete "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    ${EndIf}
    !insertmacro IsShortcutTarget "$SMPROGRAMS\${PRODUCTNAME}\${PRODUCTNAME}.lnk" "$R8\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      Delete "$SMPROGRAMS\${PRODUCTNAME}\${PRODUCTNAME}.lnk"
      RMDir "$SMPROGRAMS\${PRODUCTNAME}"
    ${EndIf}

    DeleteRegKey HKCU "${UNINSTKEY}"
    DeleteRegKey HKCU "${MANUPRODUCTKEY}"
    DeleteRegKey /ifempty HKCU "${MANUKEY}"

    ; Keep "Launch at startup" working from the new location (MooDeX also rewrites it on start).
    ReadRegStr $R9 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"
    ${If} $R9 != ""
      WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}" '"$INSTDIR\${MAINBINARYNAME}.exe" --minimized'
    ${EndIf}
  ${EndIf}
  SetShellVarContext all
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    ; The library lives in the user's profile, not the per-machine context.
    SetShellVarContext current
    ; Export first: copy the database to Documents before removing MooDeX v2 data.
    CreateDirectory "$DOCUMENTS\MooDeX backup"
    CopyFiles /SILENT "$LOCALAPPDATA\MooDeX\v2\moodex.db" "$DOCUMENTS\MooDeX backup\moodex.db"
    ; Only our own v2 folder - never the shared %LOCALAPPDATA%\MooDeX root.
    RMDir /r "$LOCALAPPDATA\MooDeX\v2"
    SetShellVarContext all
  ${EndIf}
!macroend
