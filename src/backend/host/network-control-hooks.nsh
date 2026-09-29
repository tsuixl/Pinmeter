; Remove only rules owned by this installation, before removing its recovery tool.
!macro NSIS_HOOK_PREUNINSTALL
  ; Stop the rule writer before cleanup. The standard template checks again afterwards.
  !insertmacro CheckIfAppIsRunning "${MAINBINARYNAME}.exe" "${PRODUCTNAME}"
  ; In-place upgrades keep blocks until the new version resumes the saved rules.
  StrCmp $UpdateMode 1 control_cleanup_done
  IfFileExists "$INSTDIR\network-control\pinmeter-network-control.exe" 0 control_cleanup_failed
  ClearErrors
  ; The helper inherits or requests elevation and returns its verified cleanup result.
  ExecWait '"$INSTDIR\network-control\pinmeter-network-control.exe" --uninstall-cleanup-request' $0
  IfErrors control_cleanup_failed
  StrCmp $0 0 control_cleanup_done
  control_cleanup_failed:
    MessageBox MB_OK|MB_ICONEXCLAMATION "Pinmeter could not verify network and startup cleanup. Restore the complete installation and retry uninstalling. No application files have been removed." /SD IDOK
    SetErrorLevel 1
    Abort
  control_cleanup_done:
!macroend
