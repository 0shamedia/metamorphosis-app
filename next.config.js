/** @type {import('next').NextConfig} */
const nextConfig = {
  output: 'export',
  // Optional: Add other Next.js configurations here if needed
  // For example, if you need to handle images or other assets:
  // images: {
  //   unoptimized: true,
  // },
  webpack: (config, { isServer }) => {
    // Only apply this to the client-side build
    if (!isServer) {
      config.externals.push(
        /^@tauri-apps\/api\/.+$/
      );
      config.externals.push(
        /^@tauri-apps\/plugin-.+$/
      );
    }
    return config;
  },
};

module.exports = nextConfig;