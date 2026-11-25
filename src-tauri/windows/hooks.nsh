; Hops installer hooks: keep the HKCU registry tidy.
;
; The key lists mirror OWNED_KEYS and LEGACY_KEYS in src-tauri/src/registration.rs.
; A unit test there fails if a key is missing from this file.

; Keys written by Hops 0.1.6 and earlier that are no longer used: the .html
; association and the hops: custom scheme.
!macro HOPS_DELETE_LEGACY_KEYS
  DeleteRegKey HKCU "Software\Classes\HopsHTML"
  DeleteRegKey HKCU "Software\Classes\Hops"
  DeleteRegKey HKCU "Software\Hops\Capabilities\FileAssociations"
!macroend

; SHChangeNotify(SHCNE_ASSOCCHANGED), so Default Apps reflects the change.
!macro HOPS_NOTIFY_ASSOCIATIONS_CHANGED
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend

; Runs on every install, including in-app updates.
!macro NSIS_HOOK_POSTINSTALL
  !insertmacro HOPS_DELETE_LEGACY_KEYS
  !insertmacro HOPS_NOTIFY_ASSOCIATIONS_CHANGED
!macroend

; An update runs the old uninstaller with /UPDATE: keep the registration then,
; or every update would stop Hops being the default browser.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    DeleteRegValue HKCU "Software\RegisteredApplications" "Hops"
    DeleteRegKey HKCU "Software\Classes\HopsURL"
    DeleteRegKey HKCU "Software\Hops"
    !insertmacro HOPS_DELETE_LEGACY_KEYS
    ; The template deletes the autostart Run value; auto-launch also writes this one.
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Hops"
    !insertmacro HOPS_NOTIFY_ASSOCIATIONS_CHANGED
  ${EndIf}
!macroend
