const vscode = require("vscode");
const path = require("path");

function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand("nexus.openCli", () => {
      const root = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
      if (!root) {
        vscode.window.showErrorMessage("Open a folder first");
        return;
      }
      const term = vscode.window.createTerminal({ name: "Nexus CLI", cwd: root });
      term.sendText("nexus");
      term.show();
    })
  );
  context.subscriptions.push(
    vscode.commands.registerCommand("nexus.startEngine", () => {
      const root = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
      if (!root) {
        vscode.window.showErrorMessage("Open a folder first");
        return;
      }
      const engineDir = path.join(root, "packages", "nexus-engine");
      const term = vscode.window.createTerminal("Nexus Engine");
      term.sendText(`cd "${engineDir}" && uv run nexus-engine`);
      term.show();
    })
  );
}

function deactivate() {}

module.exports = { activate, deactivate };
