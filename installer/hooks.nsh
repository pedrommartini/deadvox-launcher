; Preserve downloaded voice packs and user-data. Before uninstalling an
; installation that has saved state, restore the game's original mounts.
!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode != 1
    IfFileExists "$INSTDIR\user-data\state.json" 0 deadvox_uninstall_done
    IfFileExists "$INSTDIR\${MAINBINARYNAME}.exe" 0 deadvox_uninstall_missing
    IfSilent deadvox_uninstall_restore 0
    MessageBox MB_OKCANCEL|MB_ICONINFORMATION "Antes de remover o DeadVox, a instalação restaurará a configuração de voz original do jogo. Feche Deadlock primeiro. Seus pacotes baixados e configurações serão mantidos." IDOK deadvox_uninstall_restore
    Abort
    deadvox_uninstall_restore:
      InitPluginsDir
      ExecWait '"$INSTDIR\${MAINBINARYNAME}.exe" --check=disable --report="$PLUGINSDIR\deadvox-uninstall-check.json"' $0
      ${If} $0 != 0
        IfSilent +2 0
        MessageBox MB_ICONSTOP "DeadVox não conseguiu restaurar a configuração original do jogo. Feche o jogo, abra o launcher, selecione Original e tente desinstalar novamente. Nenhum arquivo foi removido."
        Abort
      ${EndIf}
      Goto deadvox_uninstall_done
    deadvox_uninstall_missing:
      IfSilent +2 0
      MessageBox MB_ICONSTOP "O launcher não foi encontrado; por isso não é possível verificar a configuração do jogo com segurança. Reinstale o DeadVox e selecione Original antes de desinstalar."
      Abort
    deadvox_uninstall_done:
  ${EndIf}
!macroend
