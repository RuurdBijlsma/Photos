package dev.ruurd.photos

import android.app.Application
import coil3.ImageLoader
import coil3.SingletonImageLoader
import dev.ruurd.photos.di.AppContainer
import dev.ruurd.photos.di.DefaultAppContainer

class PhotosApplication : Application(), SingletonImageLoader.Factory {

    lateinit var container: AppContainer
        private set

    override fun onCreate() {
        super.onCreate()
        container = DefaultAppContainer(this)
    }

    override fun newImageLoader(context: android.content.Context): ImageLoader {
        return container.imageLoader
    }
}
