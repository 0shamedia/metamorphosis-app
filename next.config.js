/** @type {import('next').NextConfig} */
const nextConfig = {
  // For Tauri static export
  output: 'export',
  
  // Disable image optimization for static export
  images: {
    unoptimized: true,
  },
  
  // Disable the eslint check during build
  eslint: {
    ignoreDuringBuilds: true,
  },
  
  // Disable strict mode for development
  reactStrictMode: false
};

module.exports = nextConfig;