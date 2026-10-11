package com.bylazar.panels.androidupdater

import android.app.Activity
import android.content.Intent
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.webkit.WebView
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import org.json.JSONObject
import java.io.File
import javax.net.ssl.HttpsURLConnection
import java.net.URL
import java.security.MessageDigest
import java.util.concurrent.Executors

private data class AndroidUpdate(
    val version: String,
    val versionCode: Long,
    val notes: String,
    val apkUrl: String,
    val sha256: String,
    val signingCertificateSha256: String,
    val releasePageUrl: String,
)

/** Native Android download and Package Installer bridge for signed APK releases. */
@TauriPlugin
class AndroidUpdaterPlugin(private val activity: Activity) : Plugin(activity) {
    companion object {
        private const val MANIFEST_URL =
            "https://raw.githubusercontent.com/pulsarupb/ControlHub/main/android-updates.json"
        private const val APP_RELEASE_PREFIX =
            "/pulsarupb/ControlHub/releases/download/"
        private const val MAX_MANIFEST_BYTES = 128 * 1024
        private const val MAX_APK_BYTES = 250L * 1024L * 1024L
        private const val CONNECT_TIMEOUT_MS = 15_000
        private const val READ_TIMEOUT_MS = 30_000
    }

    private val io = Executors.newSingleThreadExecutor()

    @Volatile
    private var pendingUpdate: AndroidUpdate? = null

    @Volatile
    private var downloadedApk: File? = null

    @Volatile
    private var downloading = false

    @Volatile
    private var downloadedBytes = 0L

    @Volatile
    private var totalBytes = 0L

    @Command
    fun checkUpdate(invoke: Invoke) {
        io.execute {
            try {
                val update = readUpdateManifest()
                val currentVersionCode = installedVersionCode()
                val available = update.versionCode > currentVersionCode
                if (available) {
                    if (pendingUpdate?.versionCode != update.versionCode) downloadedApk = null
                    pendingUpdate = update
                } else {
                    pendingUpdate = null
                downloadedApk = null
                }

                val installedCertificate = installedCertificateSha256()
                val result = JSObject()
                result.put("available", available)
                result.put(
                    "requiresReinstall",
                    available && !installedCertificate.equals(
                        update.signingCertificateSha256,
                        ignoreCase = true,
                    ),
                )
                if (available) {
                    result.put("version", update.version)
                    result.put("versionCode", update.versionCode)
                    result.put("notes", update.notes)
                    result.put("releasePageUrl", update.releasePageUrl)
                }
                resolve(invoke, result)
            } catch (error: Exception) {
                reject(invoke, error.message ?: "Unable to check for Android updates")
            }
        }
    }

    @Command
    fun downloadUpdate(invoke: Invoke) {
        io.execute {
            try {
                val update = pendingUpdate ?: readUpdateManifest().also {
                    if (it.versionCode <= installedVersionCode()) {
                        throw IllegalStateException("No Android update is available")
                    }
                    pendingUpdate = it
                }

                downloadedApk?.takeIf { it.isFile && it.length() > 0L }?.let { cached ->
                    verifyApk(cached, update)
                    resolve(invoke, JSObject().put("downloaded", true))
                    return@execute
                }

                ensureUpdateSigner(update)
                downloading = true
                downloadedBytes = 0L
                totalBytes = 0L

                val updateDirectory = File(activity.cacheDir, "updates")
                if (!updateDirectory.exists() && !updateDirectory.mkdirs()) {
                    throw IllegalStateException("Unable to create update cache")
                }
                val apk = File(updateDirectory, "panels-${update.versionCode}.apk")
                if (apk.exists() && !apk.delete()) {
                    throw IllegalStateException("Unable to replace the cached update")
                }

                val digest = MessageDigest.getInstance("SHA-256")
                openHttpsConnection(update.apkUrl).use { connection ->
                    totalBytes = connection.contentLengthLong.coerceAtLeast(0L)
                    require(totalBytes <= MAX_APK_BYTES) {
                        "Android update APK is too large"
                    }
                    connection.inputStream.use { input ->
                        apk.outputStream().buffered().use { output ->
                            val buffer = ByteArray(32 * 1024)
                            while (true) {
                                val count = input.read(buffer)
                                if (count < 0) break
                                require(downloadedBytes + count <= MAX_APK_BYTES) {
                                    "Android update APK is too large"
                                }
                                output.write(buffer, 0, count)
                                digest.update(buffer, 0, count)
                                downloadedBytes += count
                            }
                        }
                    }
                }

                val actualSha256 = digest.digest().toHex()
                if (!actualSha256.equals(update.sha256, ignoreCase = true)) {
                    apk.delete()
                    throw SecurityException("Downloaded APK checksum did not match the release manifest")
                }
                verifyApk(apk, update)
                downloadedApk = apk
                resolve(invoke, JSObject().put("downloaded", true))
            } catch (error: Exception) {
                reject(invoke, error.message ?: "Unable to download Android update")
            } finally {
                downloading = false
            }
        }
    }

    @Command
    fun getDownloadProgress(invoke: Invoke) {
        val progress = JSObject()
        progress.put("downloading", downloading)
        progress.put("downloadedBytes", downloadedBytes)
        progress.put("totalBytes", totalBytes)
        invoke.resolve(progress)
    }

    @Command
    fun installUpdate(invoke: Invoke) {
        try {
            val update = pendingUpdate
                ?: throw IllegalStateException("Check for an update before installing it")
            val apk = downloadedApk
                ?: throw IllegalStateException("Download the update before installing it")
            if (!apk.isFile || apk.length() == 0L) {
                throw IllegalStateException("The downloaded update is missing")
            }
            verifyApk(apk, update)

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O &&
                !activity.packageManager.canRequestPackageInstalls()
            ) {
                val settings = Intent(
                    Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                    Uri.parse("package:${activity.packageName}"),
                ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                activity.startActivity(settings)
                val result = JSObject()
                result.put("permissionRequired", true)
                result.put("installerLaunched", false)
                invoke.resolve(result)
                return
            }

            val uri = FileProvider.getUriForFile(
                activity,
                "${activity.packageName}.fileprovider",
                apk,
            )
            val installer = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, "application/vnd.android.package-archive")
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            activity.startActivity(installer)

            val result = JSObject()
            result.put("permissionRequired", false)
            result.put("installerLaunched", true)
            invoke.resolve(result)
        } catch (error: Exception) {
            invoke.reject(error.message ?: "Unable to open the Android package installer")
        }
    }

    @Command
    fun openReleasePage(invoke: Invoke) {
        try {
            val update = pendingUpdate
                ?: throw IllegalStateException("Check for an update before opening its release page")
            val intent = Intent(Intent.ACTION_VIEW, Uri.parse(update.releasePageUrl))
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            activity.startActivity(intent)
            invoke.resolve()
        } catch (error: Exception) {
            invoke.reject(error.message ?: "Unable to open the Android release page")
        }
    }

    override fun onDestroy() {
        io.shutdownNow()
    }

    private fun readUpdateManifest(): AndroidUpdate {
        val connection = openHttpsConnection(MANIFEST_URL)
        connection.use {
            if (it.contentLengthLong > MAX_MANIFEST_BYTES) {
                throw IllegalStateException("Android update manifest is unexpectedly large")
            }
            val body = it.inputStream.bufferedReader(Charsets.UTF_8).use { reader ->
                reader.readText()
            }
            if (body.toByteArray(Charsets.UTF_8).size > MAX_MANIFEST_BYTES) {
                throw IllegalStateException("Android update manifest is unexpectedly large")
            }
            val json = JSONObject(body)
            val version = json.getString("version")
            val versionCode = json.getLong("versionCode")
            val apkUrl = json.getString("apkUrl")
            val sha256 = json.getString("sha256").lowercase()
            val certificate = json.getString("signingCertificateSha256").lowercase()
            val releasePageUrl = json.getString("releasePageUrl")

            require(version.matches(Regex("\\d+\\.\\d+\\.\\d+"))) {
                "Android update version is invalid"
            }
            require(versionCode > 0) { "Android update version code is invalid" }
            require(sha256.matches(Regex("[0-9a-f]{64}"))) {
                "Android update checksum is invalid"
            }
            require(certificate.matches(Regex("[0-9a-f]{64}"))) {
                "Android signing certificate fingerprint is invalid"
            }
            require(isTrustedReleaseUrl(apkUrl)) { "Android APK URL is not a trusted release URL" }
            require(isTrustedReleasePageUrl(releasePageUrl)) {
                "Android release page URL is not trusted"
            }

            return AndroidUpdate(
                version = version,
                versionCode = versionCode,
                notes = json.optString("notes", ""),
                apkUrl = apkUrl,
                sha256 = sha256,
                signingCertificateSha256 = certificate,
                releasePageUrl = releasePageUrl,
            )
        }
    }

    private fun openHttpsConnection(start: String): HttpsConnection {
        var url = URL(start)
        for (redirects in 0..5) {
            require(url.protocol.equals("https", ignoreCase = true)) {
                "Android update URLs must use HTTPS"
            }
            require(isTrustedHost(url.host)) { "Android update host is not trusted" }
            val connection = (url.openConnection() as? HttpsURLConnection)
                ?: throw IllegalStateException("Android update URL must use HTTPS")
            connection.connectTimeout = CONNECT_TIMEOUT_MS
            connection.readTimeout = READ_TIMEOUT_MS
            connection.instanceFollowRedirects = false
            connection.setRequestProperty("User-Agent", "Panels-Android-Updater")
            connection.connect()

            if (connection.responseCode in 300..399) {
                val location = connection.getHeaderField("Location")
                    ?: throw IllegalStateException("Android update redirect is missing a location")
                connection.disconnect()
                if (redirects == 5) throw IllegalStateException("Too many Android update redirects")
                url = URL(url, location)
                continue
            }
            if (connection.responseCode !in 200..299) {
                val status = connection.responseCode
                connection.disconnect()
                throw IllegalStateException("Android update server returned HTTP $status")
            }
            return HttpsConnection(connection)
        }
        throw IllegalStateException("Too many Android update redirects")
    }

    private fun isTrustedHost(host: String): Boolean =
        host.equals("raw.githubusercontent.com", ignoreCase = true) ||
            host.equals("github.com", ignoreCase = true) ||
            host.equals("release-assets.githubusercontent.com", ignoreCase = true) ||
            host.equals("objects.githubusercontent.com", ignoreCase = true)

    private fun isTrustedReleaseUrl(value: String): Boolean = try {
        val url = URL(value)
        url.protocol.equals("https", ignoreCase = true) &&
            url.host.equals("github.com", ignoreCase = true) &&
            url.path.startsWith(APP_RELEASE_PREFIX) &&
            url.userInfo == null
    } catch (_: Exception) {
        false
    }

    private fun isTrustedReleasePageUrl(value: String): Boolean = try {
        val url = URL(value)
        url.protocol.equals("https", ignoreCase = true) &&
            url.host.equals("github.com", ignoreCase = true) &&
            url.path.startsWith("/pulsarupb/ControlHub/releases/tag/") &&
            url.userInfo == null
    } catch (_: Exception) {
        false
    }

    private fun installedVersionCode(): Long {
        val info = packageInfo(activity.packageName)
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            info.longVersionCode
        } else {
            @Suppress("DEPRECATION")
            info.versionCode.toLong()
        }
    }

    private fun installedCertificateSha256(): String =
        packageCertificateSha256(packageInfo(activity.packageName))

    private fun ensureUpdateSigner(update: AndroidUpdate) {
        if (!installedCertificateSha256().equals(update.signingCertificateSha256, ignoreCase = true)) {
            throw SecurityException(
                "This installation must be reinstalled with the signed Android release before it can update in-app",
            )
        }
    }

    private fun verifyApk(apk: File, update: AndroidUpdate) {
        val flags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            PackageManager.GET_SIGNING_CERTIFICATES
        } else {
            @Suppress("DEPRECATION")
            PackageManager.GET_SIGNATURES
        }
        val archive = activity.packageManager.getPackageArchiveInfo(apk.absolutePath, flags)
            ?: throw SecurityException("Downloaded file is not a valid Android APK")
        require(archive.packageName == activity.packageName) {
            "Downloaded APK is for a different application"
        }
        val archiveVersionCode = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            archive.longVersionCode
        } else {
            @Suppress("DEPRECATION")
            archive.versionCode.toLong()
        }
        require(archiveVersionCode == update.versionCode) {
            "Downloaded APK version does not match the update manifest"
        }
        require(packageCertificateSha256(archive).equals(
            update.signingCertificateSha256,
            ignoreCase = true,
        )) { "Downloaded APK signing certificate does not match the release manifest" }
        ensureUpdateSigner(update)
    }

    private fun packageInfo(packageName: String): PackageInfo {
        val flags = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            PackageManager.GET_SIGNING_CERTIFICATES
        } else {
            @Suppress("DEPRECATION")
            PackageManager.GET_SIGNATURES
        }
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            activity.packageManager.getPackageInfo(
                packageName,
                PackageManager.PackageInfoFlags.of(flags.toLong()),
            )
        } else {
            @Suppress("DEPRECATION")
            activity.packageManager.getPackageInfo(packageName, flags)
        }
    }

    private fun packageCertificateSha256(info: PackageInfo): String {
        val signatures = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            info.signingInfo?.apkContentsSigners
        } else {
            @Suppress("DEPRECATION")
            info.signatures
        } ?: throw SecurityException("Unable to read Android app signing certificate")
        require(signatures.isNotEmpty()) { "Android app has no signing certificate" }
        return MessageDigest.getInstance("SHA-256")
            .digest(signatures[0].toByteArray())
            .toHex()
    }

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }

    private fun resolve(invoke: Invoke, value: JSObject) {
        activity.runOnUiThread { invoke.resolve(value) }
    }

    private fun reject(invoke: Invoke, message: String) {
        activity.runOnUiThread { invoke.reject(message) }
    }

    private class HttpsConnection(private val connection: HttpsURLConnection) : AutoCloseable {
        val inputStream get() = connection.inputStream
        val contentLengthLong get() = connection.contentLengthLong
        override fun close() = connection.disconnect()
    }
}
