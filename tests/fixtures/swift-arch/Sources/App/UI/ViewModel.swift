import Foundation
import Core

final class ViewModel {
    let session = URLSession.shared
    let apiKey = ProcessInfo.processInfo.environment["API_KEY"]
}
