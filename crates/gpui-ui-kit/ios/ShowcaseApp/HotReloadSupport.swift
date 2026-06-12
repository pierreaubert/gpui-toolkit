#if targetEnvironment(simulator)
import Darwin
import Foundation

struct GPUIHotReloadManifest: Equatable {
    let dylibPath: String
    let entrySymbol: String
    let generation: UInt64

    init?(contents: String) {
        var values: [String: String] = [:]
        for line in contents.split(separator: "\n") {
            let parts = line.split(separator: "=", maxSplits: 1).map(String.init)
            if parts.count == 2 {
                values[parts[0]] = parts[1]
            }
        }
        guard let dylibPath = values["dylib_path"], !dylibPath.isEmpty,
              let entrySymbol = values["entry_symbol"], !entrySymbol.isEmpty,
              let generation = UInt64(values["generation"] ?? "0")
        else {
            return nil
        }
        self.dylibPath = dylibPath
        self.entrySymbol = entrySymbol
        self.generation = generation
    }
}

final class GPUIHotReloadController {
    private var loadedHandle: UnsafeMutableRawPointer?
    private var loadedManifest: GPUIHotReloadManifest?

    func reloadIfNeeded(manifestURL: URL) -> Bool {
        guard let contents = try? String(contentsOf: manifestURL),
              let manifest = GPUIHotReloadManifest(contents: contents)
        else {
            return false
        }
        guard loadedManifest != manifest else {
            return false
        }

        let flags = RTLD_NOW | RTLD_LOCAL
        guard let handle = dlopen(manifest.dylibPath, flags) else {
            NSLog("GPUI hot reload dlopen failed: \(String(cString: dlerror()))")
            return false
        }
        guard let symbol = dlsym(handle, manifest.entrySymbol) else {
            NSLog("GPUI hot reload dlsym failed for \(manifest.entrySymbol)")
            dlclose(handle)
            return false
        }

        if let oldHandle = loadedHandle {
            dlclose(oldHandle)
        }
        loadedHandle = handle
        loadedManifest = manifest

        let entry = unsafeBitCast(symbol, to: (@convention(c) () -> Void).self)
        entry()
        return true
    }
}
#endif
