import SwiftUI
import WebKit
import AVFoundation
import MediaPlayer

@main
struct MusicCraftApp: App {
    var body: some Scene { WindowGroup { PlayerView().ignoresSafeArea(.container, edges: .bottom) } }
}

struct PlayerView: UIViewRepresentable {
    func makeCoordinator() -> Coordinator { Coordinator() }

    func makeUIView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.allowsInlineMediaPlayback = true
        config.mediaTypesRequiringUserActionForPlayback = []
        let controller = config.userContentController
        controller.add(context.coordinator, name: "musiccraft")
        controller.addUserScript(WKUserScript(source: Coordinator.bridge, injectionTime: .atDocumentStart, forMainFrameOnly: true))
        let web = WKWebView(frame: .zero, configuration: config)
        web.isOpaque = false
        web.backgroundColor = UIColor(red: 0.08, green: 0.09, blue: 0.14, alpha: 1)
        context.coordinator.webView = web
        context.coordinator.installRemoteCommands()
        if let page = Bundle.main.url(forResource: "index", withExtension: "html", subdirectory: "Web") {
            web.loadFileURL(page, allowingReadAccessTo: page.deletingLastPathComponent())
        }
        return web
    }
    func updateUIView(_ uiView: WKWebView, context: Context) {}
    static func dismantleUIView(_ uiView: WKWebView, coordinator: Coordinator) {
        uiView.configuration.userContentController.removeScriptMessageHandler(forName: "musiccraft")
        MPRemoteCommandCenter.shared().playCommand.removeTarget(nil)
        MPRemoteCommandCenter.shared().pauseCommand.removeTarget(nil)
        MPRemoteCommandCenter.shared().nextTrackCommand.removeTarget(nil)
        MPRemoteCommandCenter.shared().previousTrackCommand.removeTarget(nil)
    }

    final class Coordinator: NSObject, WKScriptMessageHandler {
        weak var webView: WKWebView?
        static let bridge = """
        (() => {
          const send=(name,...args)=>window.webkit.messageHandlers.musiccraft.postMessage({name,args});
          window.MusicCraftAndroid={
            beginPlayback:()=>send('begin'), stopPlaybackService:()=>send('stop'),
            reportMedia:(...a)=>send('report',...a),
            search:()=>{document.getElementById('search-status').textContent='La búsqueda online todavía no está disponible en esta prueba para iPhone.';},
            chooseConsent:()=>{}, showYoutube:()=>{}
          };
        })();
        """
        func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
            guard let body = message.body as? [String: Any], let name = body["name"] as? String else { return }
            let args = body["args"] as? [Any] ?? []
            switch name {
            case "begin":
                do {
                    try AVAudioSession.sharedInstance().setCategory(.playback, mode: .default)
                    try AVAudioSession.sharedInstance().setActive(true)
                } catch { NSLog("MusicCraft audio session: %@", error.localizedDescription) }
            case "report":
                guard args.count >= 4, let title = args[0] as? String,
                      let playing = args[1] as? Bool, let elapsed = args[2] as? Double,
                      let duration = args[3] as? Double else { return }
                MPNowPlayingInfoCenter.default().nowPlayingInfo = [
                    MPMediaItemPropertyTitle: title,
                    MPMediaItemPropertyPlaybackDuration: duration,
                    MPNowPlayingInfoPropertyElapsedPlaybackTime: elapsed,
                    MPNowPlayingInfoPropertyPlaybackRate: playing ? 1.0 : 0.0
                ]
            case "stop": MPNowPlayingInfoCenter.default().nowPlayingInfo = nil
            default: break
            }
        }
        func installRemoteCommands() {
            let commands = MPRemoteCommandCenter.shared()
            for (command, action) in [(commands.playCommand, "play"), (commands.pauseCommand, "pause"), (commands.nextTrackCommand, "next"), (commands.previousTrackCommand, "previous")] {
                command.isEnabled = true
                command.addTarget { [weak self] _ in
                    DispatchQueue.main.async { self?.webView?.evaluateJavaScript("window.musiccraftMediaAction('" + action + "')") }
                    return .success
                }
            }
        }
    }
}
