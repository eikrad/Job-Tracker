!macro NSIS_HOOK_POSTINSTALL
  CreateShortcut "$DESKTOP\Job Tracker.lnk" "$INSTDIR\JobTracker.exe"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  Delete "$DESKTOP\Job Tracker.lnk"
!macroend
