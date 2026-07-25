Var FormationLapLegacyRunValue
Var FormationLapRestoreLegacyRunValue

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    StrCpy $FormationLapRestoreLegacyRunValue "0"
    ReadRegStr $FormationLapLegacyRunValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Formation Lap"
    ${If} $FormationLapLegacyRunValue != ""
    ${AndIf} $FormationLapLegacyRunValue != "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --minimized"
    ${AndIf} $FormationLapLegacyRunValue != "$\"\\?\$INSTDIR\${MAINBINARYNAME}.exe$\" --minimized"
      ; Tauri's default template removes the product-name value unconditionally.
      ; Preserve a foreign value here and restore it in the post-uninstall hook.
      StrCpy $FormationLapRestoreLegacyRunValue "1"
    ${EndIf}

    ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "com.formationlap.desktop.StartWithWindows.v1"
    ${If} $0 == "$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --minimized"
    ${OrIf} $0 == "$\"\\?\$INSTDIR\${MAINBINARYNAME}.exe$\" --minimized"
      DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "com.formationlap.desktop.StartWithWindows.v1"
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
  ${AndIf} $FormationLapRestoreLegacyRunValue == "1"
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Formation Lap" $FormationLapLegacyRunValue
  ${EndIf}
!macroend
