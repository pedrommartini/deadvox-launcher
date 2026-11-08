using System;
using System.IO;
using System.Diagnostics;
using System.Reflection;
using System.Globalization;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Markup;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Media.Imaging;

namespace DeadVoxSetup {
 static class Program {
  static Window window; static int page; static bool busy, preview; static string destination, uiLanguage;
  static T UI<T>(string name) where T:class { return window.FindName(name) as T; }
  static Stream Resource(string name) { return Assembly.GetExecutingAssembly().GetManifestResourceStream(name); }
  [STAThread] static void Main(string[] args) {
   preview=Array.IndexOf(args,"--preview")>=0;
   uiLanguage=DetectUiLanguage();
   CultureInfo.DefaultThreadCurrentCulture=CultureInfo.GetCultureInfo(uiLanguage);
   CultureInfo.DefaultThreadCurrentUICulture=CultureInfo.GetCultureInfo(uiLanguage);
   if(Array.IndexOf(args,"--silent")>=0) {
    try {
     string folder=null; foreach(var arg in args) if(arg.StartsWith("--destination=")) folder=arg.Substring(14);
     destination=ValidatePath(folder); var code=RunEngine(destination,Array.IndexOf(args,"--no-shortcut")<0); if(code==0) SaveLauncherLanguage(destination); Environment.ExitCode=code;
    } catch { Environment.ExitCode=1; } return;
   }
   try {
    using(var stream=Resource("Setup.xaml")) window=(Window)XamlReader.Load(stream);
    window.Icon=LoadImage("Icon.png"); UI<Image>("Emblem").Source=LoadImage("Icon.png"); UI<Image>("Wordmark").Source=LoadImage("Wordmark.png");
    UI<TextBox>("Location").Text=Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),"DeadVox");
    if(preview) UI<TextBlock>("Footer").Text="PRÉVIA VISUAL";
    UI<Border>("TitleBar").MouseLeftButtonDown+=(s,e)=> { if(e.ClickCount==1) window.DragMove(); };
    UI<Button>("Minimize").Click+=(s,e)=>window.WindowState=WindowState.Minimized;
    UI<Button>("Close").Click+=(s,e)=>window.Close();
    window.Closing+=(s,e)=> { if(busy) e.Cancel=true; };
    UI<Button>("Browse").Click+=(s,e)=> {
     using(var dialog=new System.Windows.Forms.FolderBrowserDialog()) {
      dialog.Description="Escolha onde instalar o DeadVox"; dialog.SelectedPath=UI<TextBox>("Location").Text;
      if(dialog.ShowDialog()==System.Windows.Forms.DialogResult.OK) UI<TextBox>("Location").Text=dialog.SelectedPath;
     }
    };
    UI<Button>("Back").Click+=(s,e)=>Show(page==4 ? 1 : 0);
    UI<Button>("Next").Click+=async(s,e)=> {
     if(page==0) Show(1);
     else if(page==1) { if(Validate()) await Install(); }
     else if(page==3) {
      if(!preview && UI<CheckBox>("Launch").IsChecked==true) {
       try { Process.Start(new ProcessStartInfo(Path.Combine(destination,"DeadVox.exe")){WorkingDirectory=destination,UseShellExecute=true}); }
       catch(Exception ex) { MessageBox.Show(window,"DeadVox foi instalado, mas não pôde ser aberto. Abra-o pela pasta de instalação.\n\n"+ex.Message,"DeadVox"); }
      }
      window.Close();
     } else if(page==4) Show(1);
    };
    Show(0); new Application().Run(window);
   } catch(Exception ex) { MessageBox.Show("A instalação do DeadVox não pôde ser iniciada.\n\n"+ex.Message,"Instalação do DeadVox"); Environment.ExitCode=1; }
  }
  static BitmapImage LoadImage(string name) {
   using(var s=Resource(name)) { var b=new BitmapImage(); b.BeginInit(); b.CacheOption=BitmapCacheOption.OnLoad; b.StreamSource=s; b.EndInit(); b.Freeze(); return b; }
  }
  static void Show(int next) {
   page=next; string[] pages={"Welcome","Destination","Progress","Complete","Failure"};
   for(int i=0;i<pages.Length;i++) UI<StackPanel>(pages[i]).Visibility=i==next?Visibility.Visible:Visibility.Collapsed;
   UI<Button>("Back").Visibility=next==1||next==4?Visibility.Visible:Visibility.Collapsed;
   UI<Button>("Next").Visibility=next==2?Visibility.Collapsed:Visibility.Visible;
   UI<Button>("Next").Content=next==3?"Concluir":next==4?"Tentar novamente":"Instalar";
   UI<Button>("Close").IsEnabled=!busy;
   if(SystemParameters.ClientAreaAnimation) {
    var ease=new CubicEase{EasingMode=EasingMode.EaseOut};
    UI<Grid>("Page").BeginAnimation(UIElement.OpacityProperty,new DoubleAnimation(0,1,TimeSpan.FromMilliseconds(280)));
    UI<TranslateTransform>("PageOffset").BeginAnimation(TranslateTransform.YProperty,new DoubleAnimation(14,0,TimeSpan.FromMilliseconds(320)){EasingFunction=ease});
   }
   var spinner=(RotateTransform)UI<System.Windows.Shapes.Path>("Spinner").RenderTransform;
   spinner.BeginAnimation(RotateTransform.AngleProperty,next==2?new DoubleAnimation(0,360,TimeSpan.FromSeconds(1.3)){RepeatBehavior=RepeatBehavior.Forever}:null);
  }
  static bool Validate() {
   try {
    destination=ValidatePath(UI<TextBox>("Location").Text);
    // The embedded installer also checks existing files and registered installs.
    UI<TextBlock>("Validation").Text=""; return true;
   } catch(Exception ex) { UI<TextBlock>("Validation").Text=ex.Message; return false; }
  }
  static string ValidatePath(string value) {
   value=(value??"").Trim();
   if(value.Length<4 || value.IndexOf('"')>=0 || value.IndexOf('\n')>=0 || value.StartsWith(@"\\") || !Path.IsPathRooted(value)) throw new Exception("Escolha uma pasta exclusiva em uma unidade local.");
   var full=Path.GetFullPath(value).TrimEnd(Path.DirectorySeparatorChar);
   if(full.Length<=3 || !Directory.Exists(Path.GetPathRoot(full))) throw new Exception("Escolha uma pasta exclusiva em uma unidade local disponível.");
   return full;
  }
  static int RunEngine(string folder,bool shortcut) {
   var temporary=Path.Combine(Path.GetTempPath(),"DeadVox-Setup-"+Guid.NewGuid().ToString("N")); Directory.CreateDirectory(temporary);
   try {
    string payload=Path.Combine(temporary,"setup-engine.exe");
    using(var source=Resource("Engine.exe")) using(var output=new FileStream(payload,FileMode.CreateNew,FileAccess.Write)) source.CopyTo(output);
    var start=new ProcessStartInfo(payload,"/S "+(shortcut?"":"/NS ")+"/D="+folder){UseShellExecute=false,CreateNoWindow=true,WorkingDirectory=temporary};
    using(var process=Process.Start(start)) { process.WaitForExit(); return process.ExitCode; }
   } finally { try { File.Delete(Path.Combine(temporary,"setup-engine.exe")); Directory.Delete(temporary,false); } catch { } }
  }
  static async Task Install() {
   busy=true; Show(2); bool shortcut=UI<CheckBox>("Shortcut").IsChecked==true;
   try {
    if(preview) { UI<TextBlock>("ProgressText").Text="Prévia visual — nenhum arquivo está sendo instalado."; await Task.Delay(4500); }
    else {
     UI<TextBlock>("ProgressText").Text="Instalando o launcher e verificando o WebView2...";
     int code=await Task.Run(()=>RunEngine(destination,shortcut));
     if(code!=0) throw new Exception(code==12 ? "Esta pasta contém arquivos de outra instalação. Escolha outra pasta para manter esses arquivos seguros." : "A instalação não pôde ser concluída (código "+code+"). Feche qualquer janela aberta do DeadVox e tente novamente. Confira se a pasta permite gravação e se há conexão com a Internet para o WebView2.");
     if(!File.Exists(Path.Combine(destination,"DeadVox.exe"))) throw new Exception("A instalação terminou sem o arquivo esperado do launcher. Escolha outra pasta e tente novamente.");
     SaveLauncherLanguage(destination);
    }
    busy=false; Show(3);
   } catch(Exception ex) { busy=false; UI<TextBlock>("ErrorText").Text=ex.Message; Show(4); }
  }
  static string DetectUiLanguage() {
   // The interface currently ships in pt-BR. Read Windows' UI locale so the
   // first-run choice follows the system when Portuguese is present; pt-BR is
   // the intentional fallback until another interface translation is shipped.
   var current=CultureInfo.CurrentUICulture==null ? "" : CultureInfo.CurrentUICulture.Name;
   var installed=CultureInfo.InstalledUICulture==null ? "" : CultureInfo.InstalledUICulture.Name;
   if(current.StartsWith("pt",StringComparison.OrdinalIgnoreCase)||installed.StartsWith("pt",StringComparison.OrdinalIgnoreCase)) return "pt-BR";
   return "pt-BR";
  }
  static void SaveLauncherLanguage(string folder) {
   var data=Path.Combine(folder,"user-data"); Directory.CreateDirectory(data);
   var preference=Path.Combine(data,"ui-language.json");
   if(!File.Exists(preference)) File.WriteAllText(preference,"{\"language\":\""+uiLanguage+"\"}");
  }
 }
}
