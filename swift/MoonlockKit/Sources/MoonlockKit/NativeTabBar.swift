#if os(iOS)
import UIKit

// System UITabBar laid over the Tauri webview: on iOS 26+ UIKit renders it as real Liquid Glass,
// which CSS backdrop-filter can't imitate (no refraction/lensing in WebKit).

public typealias TabCallback = @convention(c) (Int32) -> Void

private struct Tab: Decodable { let title: String; let icon: String }

private final class Host: NSObject, UITabBarDelegate {
    let bar = UITabBar()
    let onSelect: TabCallback
    init(_ cb: @escaping TabCallback) { onSelect = cb }
    func tabBar(_ tabBar: UITabBar, didSelect item: UITabBarItem) { onSelect(Int32(item.tag)) }
}

private var host: Host?

/// Must be called on the main thread. Idempotent.
@_cdecl("moonlock_tabbar_install")
public func moonlockTabbarInstall(_ vc: UnsafeMutableRawPointer, _ json: UnsafePointer<CChar>, _ cb: TabCallback) -> Bool {
    if host != nil { return true }
    guard let data = String(cString: json).data(using: .utf8),
          let tabs = try? JSONDecoder().decode([Tab].self, from: data) else { return false }
    let controller = Unmanaged<UIViewController>.fromOpaque(vc).takeUnretainedValue()
    guard let root = controller.view else { return false }

    let h = Host(cb)
    let bar = h.bar
    bar.delegate = h
    bar.overrideUserInterfaceStyle = .dark
    bar.tintColor = UIColor(red: 0.36, green: 0.60, blue: 1.0, alpha: 1)
    bar.items = tabs.enumerated().map { i, t in
        UITabBarItem(title: t.title, image: UIImage(systemName: t.icon), tag: i)
    }
    bar.selectedItem = bar.items?.first
    bar.translatesAutoresizingMaskIntoConstraints = false
    root.addSubview(bar)
    NSLayoutConstraint.activate([
        bar.leadingAnchor.constraint(equalTo: root.leadingAnchor),
        bar.trailingAnchor.constraint(equalTo: root.trailingAnchor),
        bar.bottomAnchor.constraint(equalTo: root.bottomAnchor),
    ])
    host = h
    return true
}

@_cdecl("moonlock_tabbar_select")
public func moonlockTabbarSelect(_ index: Int32) {
    guard let bar = host?.bar, let items = bar.items, items.indices.contains(Int(index)) else { return }
    bar.selectedItem = items[Int(index)]
}
#endif
