export function getChartConfig(params: {
  symbol: string;
  datafeed: any;
  container: HTMLElement;
  containerWidth: number;
  containerHeight: number;
  isMobile: boolean;
  autosize?: boolean;
}) {
  const { symbol, datafeed, container, containerWidth, containerHeight, isMobile, autosize } = params;

  return {
    symbol,
    datafeed,
    interval: '240',
    container,
    library_path: '/charting_library/charting_library/',
    width: containerWidth,
    height: isMobile ? 300 : containerHeight,
    locale: 'en',
    fullscreen: false,
    autosize: autosize ?? true,
    theme: 'dark',
    timezone: 'Etc/UTC',
    debug: true,
    disabled_features: [
      'use_localstorage_for_settings',
      'study_templates',
      'header_saveload',
      'header_settings',
      'header_compare',
      'header_symbol_search',
      'header_screenshot',
      'timeframes_toolbar',
      'symbol_info',
      ...(isMobile ? [
        'left_toolbar',
        'volume_force_overlay',
        'create_volume_indicator_by_default',
        'legend_widget',
        'border_around_the_chart',
        'main_series_scale_menu',
        'scales_date_format_button',
        'display_market_status',
        'control_bar',
      ] : [])
    ],
    enabled_features: [
      ...(isMobile ? [] : [
        'create_volume_indicator_by_default',
        'left_toolbar',
        'volume_force_overlay'
      ]),
      'show_chart_property_page',
      'support_multicharts',
      'legend_widget'
    ],
    custom_css_url: '/tradingview-chart.css',
    loading_screen: { 
      backgroundColor: "#131722",
      foregroundColor: "#2962FF"
    },
    overrides: {
      // Chart styling
      "mainSeriesProperties.candleStyle.upColor": "#22c55e",
      "mainSeriesProperties.candleStyle.downColor": "#ef4444",
      "mainSeriesProperties.candleStyle.borderUpColor": "#22c55e",
      "mainSeriesProperties.candleStyle.borderDownColor": "#ef4444",
      "mainSeriesProperties.candleStyle.wickUpColor": "#22c55e",
      "mainSeriesProperties.candleStyle.wickDownColor": "#ef4444",
      
      // Chart background
      "paneProperties.background": "rgba(0,0,0,0)",
      "paneProperties.backgroundType": "solid",
      "paneProperties.vertGridProperties.color": "rgba(30, 41, 59, 0.64)",
      "paneProperties.horzGridProperties.color": "rgba(30, 41, 59, 0.64)",
      
      // Chart area
      "chartProperties.background": "rgba(0,0,0,0)",
      "chartProperties.backgroundType": "solid",
      
      // Price scale formatting
      "mainSeriesProperties.priceFormat.precision": isMobile ? 3 : 4,
      "mainSeriesProperties.priceFormat.minMove": isMobile ? 0.001 : 0.0001,
      
      // Price axis
      "scalesProperties.backgroundColor": "rgba(0,0,0,0)",
      "scalesProperties.lineColor": "rgba(30, 41, 59, 0.2)",
      "scalesProperties.textColor": "#9ca3af",
      "scalesProperties.fontSize": isMobile ? 10 : 12,
      
      // Time axis
      "timeScale.backgroundColor": "rgba(0,0,0,0)",
      "timeScale.borderColor": "rgba(30, 41, 59, 0.2)",
      "timeScale.textColor": "#9ca3af",
      
      // Volume
      "volumePaneSize": "medium",
      ...(isMobile && {
        'paneProperties.legendProperties.showStudyArguments': false,
        'paneProperties.legendProperties.showStudyTitles': false,
        'scalesProperties.fontSize': 10,
        'timeScale.fontSize': 10
      }),
      ...(isMobile ? {
        "paneProperties.topMargin": 5,
        "paneProperties.bottomMargin": 5,
        "paneProperties.leftAxisMargin": 5,
        "paneProperties.rightAxisMargin": 5,
        
        "scalesProperties.fontSize": 10,
        "scalesProperties.textColor": "#9ca3af",
        "scalesProperties.lineColor": "rgba(255, 255, 255, 0.1)",
        
        "timeScale.fontSize": 10,
        "timeScale.rightOffset": 3,
        "timeScale.leftOffset": 3,
        "timeScale.spacingPercentage": 0.4,
        
        "mainSeriesProperties.candleStyle.drawWick": true,
        "mainSeriesProperties.candleStyle.drawBorder": true,
        "mainSeriesProperties.candleStyle.borderUpColor": "#22c55e",
        "mainSeriesProperties.candleStyle.borderDownColor": "#ef4444",
        "mainSeriesProperties.candleStyle.wickUpColor": "#22c55e",
        "mainSeriesProperties.candleStyle.wickDownColor": "#ef4444",
        
        "paneProperties.legendProperties.showLegend": false,
        "paneProperties.legendProperties.showStudyArguments": false,
        "paneProperties.legendProperties.showStudyTitles": false,
        
        "volumePaneSize": "tiny",
        
        "paneProperties.vertGridProperties.color": "rgba(255, 255, 255, 0.1)",
        "paneProperties.horzGridProperties.color": "rgba(255, 255, 255, 0.1)",
        
        "crossHairProperties.color": "#9ca3af",
        "crossHairProperties.width": 0.5,
        "crossHairProperties.style": 2,
      } : {}),
    },
    studies_overrides: {
      "volume.volume.color.0": "#ef4444",
      "volume.volume.color.1": "#22c55e",
      "volume.volume.transparency": 50,
      "volume.volume ma.color": "#2962FF",
      "volume.volume ma.transparency": 30,
      "volume.volume ma.linewidth": 2,
      "volume.show ma": true,
      "volume.ma length": 20
    }
  };
} 